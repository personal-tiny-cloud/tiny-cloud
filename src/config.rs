// This file is part of the Tiny Cloud project.
// You can find the source code of every repository here:
//		https://github.com/personal-tiny-cloud
//
// Copyright (C) 2024  hex0x0000
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// Email: hex0x0000@protonmail.com

use serde::{Deserialize, Serialize};
use std::env::current_exe;
use tcloud_library::toml;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::OnceCell;

pub static CONFIG: OnceCell<Config> = OnceCell::const_new();

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Server {
    pub host: String,
    pub port: u16,
    pub workers: usize,
    pub is_behind_proxy: bool,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[cfg(not(feature = "no-tls"))]
pub struct Tls {
    pub privkey_path: String,
    pub cert_path: String,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Registration {
    pub token_duration_seconds: u64,
    pub token_size: u8,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Logging {
    pub log_level: String,
    #[cfg(feature = "normal-log")]
    pub terminal: bool,
    #[cfg(feature = "normal-log")]
    pub file: Option<String>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct CredentialSize {
    pub max_username: u8,
    pub min_username: u8,
    pub max_passwd: u16,
    pub min_passwd: u16,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Durations {
    pub cookie_minutes: u32,
    pub login_minutes: Option<u64>,
    pub visit_minutes: Option<u64>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Limits {
    pub file_upload_size: usize,
    pub payload_size: usize,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server_name: String,
    pub description: String,
    pub url_prefix: String,
    pub server: Server,
    pub logging: Logging,
    #[cfg(not(feature = "no-tls"))]
    pub tls: Option<Tls>,
    pub registration: Option<Registration>,
    pub data_directory: String,
    pub session_secret_key_path: String,
    pub limits: Limits,
    pub duration: Durations,
    pub cred_size: CredentialSize,
    pub plugins: toml::Table,
}

fn get_exec_dir() -> Result<String, String> {
    let mut path = current_exe().map_err(|e| format!("Failed to get executable's path: {}", e))?;
    path.pop();
    Ok(path.to_str().ok_or("Failed to get executable's path")?.into())
}

impl Config {
    pub fn default(plugins: toml::Table) -> Result<Self, String> {
        Ok(Self {
            description: env!("CARGO_PKG_DESCRIPTION").to_string(),
            server_name: "Tiny Cloud".into(),
            url_prefix: "tcloud".into(),
            server: Server {
                host: "127.0.0.1".into(),
                port: 80,
                workers: num_cpus::get(),
                is_behind_proxy: false,
            },
            logging: {
                #[cfg(feature = "normal-log")]
                {
                    Logging {
                        log_level: "info".into(),
                        terminal: true,
                        file: None,
                    }
                }
                #[cfg(not(feature = "normal-log"))]
                {
                    Logging { log_level: "warn".into() }
                }
            },
            #[cfg(any(feature = "rustls", feature = "openssl"))]
            tls: Some(Tls {
                privkey_path: format!("{}/privkey.pem", get_exec_dir()?),
                cert_path: format!("{}/cert.pem", get_exec_dir()?),
            }),
            registration: Some(Registration {
                token_size: 16,
                token_duration_seconds: 24 * 60 * 60,
            }),
            data_directory: format!("{}/data", get_exec_dir()?),
            limits: Limits {
                file_upload_size: 5_000_000_000,
                payload_size: 4096,
            },
            duration: Durations {
                cookie_minutes: 43200,
                login_minutes: Some(43200),
                visit_minutes: Some(21600),
            },
            session_secret_key_path: format!("{}/secret.key", get_exec_dir()?),
            cred_size: CredentialSize {
                max_username: 10,
                min_username: 3,
                max_passwd: 256,
                min_passwd: 9,
            },
            plugins,
        })
    }
}

pub async fn open(path: String) -> Result<(), String> {
    let mut file = File::open(&path)
        .await
        .map_err(|e| format!("Failed to open config file `{path}`: {e}"))?;
    let mut config = String::new();
    file.read_to_string(&mut config)
        .await
        .map_err(|e| format!("Failed to read config file `{path}`: {e}"))?;
    CONFIG
        .set(toml::from_str(&config).map_err(|e| format!("Failed to read config file `{path}`: {e}"))?)
        .expect("Config has already been opened. This is a bug");
    Ok(())
}

pub async fn write_default(plugins: toml::Table) -> Result<(), String> {
    let mut path = current_exe().map_err(|e| format!("Failed to get executable's path: {e}"))?;
    path.pop();
    path.push("default.toml");
    let mut file = File::create(path).await.map_err(|e| format!("Failed to create config file: {e}"))?;
    let default = Config::default(plugins)?;
    let default = toml::to_string(&default).map_err(|e| format!("Failed to serialize config: {e}"))?;
    file.write_all(default.as_bytes())
        .await
        .map_err(|e| format!("Failed to write config: {e}"))?;
    Ok(())
}

pub fn get() -> &'static Config {
    CONFIG
        .get()
        .expect("Tried to access config while it wasn't opened yet. This is a bug")
}
