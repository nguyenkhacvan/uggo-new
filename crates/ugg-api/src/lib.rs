use anyhow::Result;
use reqwest::Client;
use std::collections::HashMap;
use ugg_types::{
    mappings::{Mode, Region, Role},
    matchups::MatchupData,
    overview::Overview,
};

pub struct UggApi {
    client: Client,
    pub api_version: String,
}

impl UggApi {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36")
            .build()
            .unwrap_or_default();

        Self {
            client,
            api_version: "15.1.1".to_string(), // Mặc định, sẽ được cập nhật sau
        }
    }

    /// Lấy version mới nhất từ DDragon (Async)
    pub async fn fetch_current_version(&mut self) -> Result<String> {
        let versions: Vec<String> = self
            .client
            .get("https://ddragon.leagueoflegends.com/api/versions.json")
            .send()
            .await?
            .json()
            .await?;

        if let Some(ver) = versions.first() {
            self.api_version = ver.clone();
        }
        Ok(self.api_version.clone())
    }

    /// Lấy Overview (Build, Runes) của tướng (Async)
    pub async fn get_overview(
        &self,
        champ: &str,
        mode: Mode,
        role: Role,
        region: Region,
    ) -> Result<Box<Overview>> {
        let region_query = if region == Region::World {
            "".to_string()
        } else {
            format!("&regionId={}", region as i32)
        };

        let role_query = if role == Role::Automatic {
            "".to_string()
        } else {
            format!("&roleId={}", role as i32)
        };

        let url = format!(
            "https://stats2.u.gg/lol/1.5/overview/{}/{}/{}/{}.json?{}",
            self.api_version,
            mode.to_api_string(),
            champ,
            "1.5.0", // Hardcoded minor version for UGG API consistency
            format!("{}{}", region_query, role_query)
        );

        let response = self.client.get(&url).send().await?;
        let overview = response.json::<Overview>().await?;
        
        Ok(Box::new(overview))
    }

    /// Lấy Matchups (Async)
    pub async fn get_matchups(
        &self,
        champ: &str,
        mode: Mode,
        role: Role,
        region: Region,
    ) -> Result<Box<MatchupData>> {
        let region_query = if region == Region::World {
            "".to_string()
        } else {
            format!("&regionId={}", region as i32)
        };

        let role_query = if role == Role::Automatic {
            "".to_string()
        } else {
            format!("&roleId={}", role as i32)
        };

        let url = format!(
            "https://stats2.u.gg/lol/1.5/matchups/{}/{}/{}/{}.json?{}",
            self.api_version,
            mode.to_api_string(),
            champ,
            "1.5.0",
            format!("{}{}", region_query, role_query)
        );

        let response = self.client.get(&url).send().await?;
        let matchups = response.json::<MatchupData>().await?;

        Ok(Box::new(matchups))
    }
}
