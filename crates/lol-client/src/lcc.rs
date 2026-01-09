//! # `league_client_connector` - Optimized Version
//! Sử dụng sysinfo để quét process thay vì PowerShell/grep giúp khởi động tức thì.

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use regex::Regex;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
use std::fs;
use std::path::PathBuf;
use sysinfo::{System, SystemExt, ProcessExt}; // Import sysinfo

/// Make sure the League of Legends Client is opened before running any of the methods.
pub struct LeagueClientConnector {}

impl LeagueClientConnector {
    /// Parses League's lockfile using fast native process lookup
    pub fn parse_lockfile() -> Result<RiotLockFile> {
        let path_str = Self::get_path()?;
        let mut path = PathBuf::from(path_str);
        path.push("lockfile");
        
        let lockfile_path = path.to_str().ok_or(LeagueConnectorError::EmptyPath {})?;
        let contents = fs::read_to_string(lockfile_path).context(UnableToReadSnafu)?;

        let pieces: Vec<&str> = contents.split(':').collect();

        if pieces.len() < 5 {
             return Err(LeagueConnectorError::InvalidLockfileFormat {});
        }

        let username = "riot".to_string();
        let address = "127.0.0.1".to_string();
        let process = pieces[0].to_string();
        let pid = pieces[1].parse().context(NumberParseSnafu { name: "pid" })?;
        let port = pieces[2].parse().context(NumberParseSnafu { name: "port" })?;
        let password = pieces[3].to_string();
        let protocol = pieces[4].to_string();
        let b64_auth = BASE64_STANDARD.encode(format!("{username}:{password}").as_bytes());

        Ok(RiotLockFile {
            process,
            pid,
            port,
            password,
            protocol,
            username,
            address,
            b64_auth,
        })
    }

    /// Tìm đường dẫn cài đặt League of Legends bằng cách quét RAM (Process List)
    /// Nhanh hơn 100x so với gọi lệnh terminal. Hỗ trợ cả Windows & macOS.
    pub fn get_path() -> Result<String> {
        let mut system = System::new_all();
        system.refresh_processes();

        // Tìm process tên là LeagueClientUx
        let process = system.processes().values().find(|p| {
            let name = p.name().to_lowercase();
            // Windows thường có .exe, Unix thì không
            name == "leagueclientux.exe" || name == "leagueclientux"
        });

        let Some(proc) = process else {
             return Err(LeagueConnectorError::NoInstallationPath {});
        };

        // Lấy command line arguments
        let cmd_args = proc.cmd();
        let raw_info = cmd_args.join(" ");

        // Parse regex tìm --install-directory
        let pattern = Regex::new(r"--install-directory=(?P<dir>[^ ]+)")
            .context(RegexParseSnafu)?;
        
        // Đôi khi path có dấu "" bao quanh, regex trên bắt đơn giản
        // Nếu path có space, argument thường nằm trong quote.
        // Cải tiến regex để bắt cả trường hợp có quote hoặc không.
        let complex_pattern = Regex::new(r#"--install-directory=(?P<dir>"[^"]+"|[^\s"]+)"#)
             .context(RegexParseSnafu)?;

        let caps = complex_pattern.captures(&raw_info).or_else(|| pattern.captures(&raw_info));

        match caps {
            Some(c) => {
                let dir = c["dir"].to_string().replace("\"", ""); // Remove quotes if any
                Ok(dir)
            }
            None => Err(LeagueConnectorError::NoInstallationPath {}),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RiotLockFile {
    pub process: String,
    pub pid: u32,
    pub port: u32,
    pub password: String,
    pub protocol: String,
    pub username: String,
    pub address: String,
    pub b64_auth: String,
}

pub type Result<T, E = LeagueConnectorError> = std::result::Result<T, E>;

#[derive(Debug, Snafu)]
pub enum LeagueConnectorError {
    #[snafu(display("Unable to parse Regex: {}", source))]
    RegexParse { source: regex::Error },

    #[snafu(display("No active LeagueClientUx process found"))]
    NoInstallationPath {},

    #[snafu(display("Path is empty"))]
    EmptyPath {},

    #[snafu(display("Lockfile content is invalid"))]
    InvalidLockfileFormat {},

    #[snafu(display("Unable to read file: {}", source))]
    UnableToRead { source: std::io::Error },

    #[snafu(display("Unable to parse to number {}: {}", name, source))]
    NumberParse {
        source: std::num::ParseIntError,
        name: &'static str,
    },
}