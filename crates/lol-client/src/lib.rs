use lcc::RiotLockFile;
use reqwest::{
    header::{self, HeaderMap, HeaderValue},
    Client, StatusCode,
};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;
use thiserror::Error;
use ugg_types::{
    client_runepage::{NewRunePage, RunePage},
    client_summoner::Summoner,
};

pub mod lcc;

pub struct LeagueClient {
    client: Client,
    base_url: String,
}

#[derive(Error, Debug)]
pub enum LeagueClientError {
    #[snafu(display("Reqwest error: {}", source))]
    #[error("Reqwest error: {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("API returned error code: {0}")]
    ApiError(StatusCode),
}

type Result<T> = std::result::Result<T, LeagueClientError>;

impl LeagueClient {
    pub fn new(lockfile: RiotLockFile) -> Result<Self> {
        let mut headers = HeaderMap::new();
        
        // Setup Authorization Header: "Basic <base64_auth>"
        let auth_value = format!("Basic {}", lockfile.b64_auth);
        let mut auth_header = HeaderValue::from_str(&auth_value)
            .map_err(|_| reqwest::Error::from(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid Auth Header")))?;
        auth_header.set_sensitive(true);
        headers.insert(header::AUTHORIZATION, auth_header);

        // Setup Client với cấu hình đặc biệt cho LCU
        let client = Client::builder()
            .default_headers(headers)
            // QUAN TRỌNG: LCU dùng self-signed cert, phải tắt verify
            .danger_accept_invalid_certs(true)
            .user_agent("uggo-lol-client/0.5.1")
            .timeout(Duration::from_secs(10))
            .build()?;

        Ok(Self {
            client,
            base_url: format!("{}://127.0.0.1:{}", lockfile.protocol, lockfile.port),
        })
    }

    /// Helper nội bộ để thực hiện GET request
    async fn get<T: DeserializeOwned>(&self, endpoint: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url, endpoint);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(LeagueClientError::ApiError(response.status()));
        }

        Ok(response.json::<T>().await?)
    }

    /// Helper nội bộ để thực hiện DELETE request
    async fn delete(&self, endpoint: &str) -> Result<()> {
        let url = format!("{}{}", self.base_url, endpoint);
        let response = self.client.delete(&url).send().await?;

        if !response.status().is_success() {
            return Err(LeagueClientError::ApiError(response.status()));
        }
        Ok(())
    }

    /// Helper nội bộ để thực hiện POST request
    async fn post<T: Serialize, R: DeserializeOwned>(&self, endpoint: &str, body: &T) -> Result<R> {
        let url = format!("{}{}", self.base_url, endpoint);
        let response = self.client.post(&url).json(body).send().await?;

        if !response.status().is_success() {
            return Err(LeagueClientError::ApiError(response.status()));
        }

        Ok(response.json::<R>().await?)
    }

    // --- API Methods (Mapped to LCU Endpoints) ---

    pub async fn get_current_summoner(&self) -> Result<Summoner> {
        self.get("/lol-summoner/v1/current-summoner").await
    }

    pub async fn get_rune_pages(&self) -> Result<Vec<RunePage>> {
        self.get("/lol-perks/v1/pages").await
    }

    pub async fn get_current_rune_page(&self) -> Result<RunePage> {
        self.get("/lol-perks/v1/currentpage").await
    }

    pub async fn delete_rune_page(&self, id: i64) -> Result<()> {
        self.delete(&format!("/lol-perks/v1/pages/{}", id)).await
    }

    pub async fn create_rune_page(&self, page: &NewRunePage) -> Result<RunePage> {
        self.post("/lol-perks/v1/pages", page).await
    }
}
