use crate::config;
use serde::{Deserialize, Serialize};
use std::env::current_exe;
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::OnceCell;

pub static CONFIG: OnceCell<Config> = OnceCell::const_new();

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Server {
    pub host: String,
    pub port: u16,
    pub workers: usize,
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
    pub max_accounts: usize,
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
pub struct Config {
    pub server_name: String,
    pub description: String,
    pub url_prefix: String,
    pub server: Server,
    pub logging: Logging,
    #[cfg(not(feature = "no-tls"))]
    pub tls: Tls,
    pub registration: Option<Registration>,
    pub max_account_storage_bytes: usize,
    pub data_directory: String,
    pub cookie_duration_minutes: u32,
    pub login_deadline_minutes: Option<u64>,
    pub visit_deadline_minutes: Option<u64>,
    pub session_secret_key_path: String,
    pub max_username_size: u8,
    pub min_username_size: u8,
    pub max_passwd_size: u16,
    pub min_passwd_size: u16,
}

fn get_exec_dir() -> Result<String, String> {
    let mut path = current_exe().map_err(|e| format!("Failed to get executable's path: {}", e))?;
    path.pop();
    Ok(path
        .to_str()
        .ok_or_else(|| format!("Failed to get executable's path"))?
        .to_string())
}

impl Config {
    pub fn default() -> Result<Self, String> {
        Ok(Self {
            description: env!("CARGO_PKG_DESCRIPTION").to_string(),
            server_name: "Tiny Cloud".into(),
            url_prefix: "tcloud".into(),
            server: Server {
                host: "127.0.0.1".into(),
                port: 80,
                workers: num_cpus::get(),
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
                    Logging {
                        log_level: "warn".into(),
                    }
                }
            },
            #[cfg(any(feature = "rustls", feature = "openssl", feature = "openssl-bundled"))]
            tls: Tls {
                privkey_path: format!("{}/privkey.pem", get_exec_dir()?),
                cert_path: format!("{}/cert.pem", get_exec_dir()?),
            },
            registration: Some(Registration {
                token_size: 16,
                token_duration_seconds: 24 * 60 * 60,
                max_accounts: 50,
            }),
            max_account_storage_bytes: 10485750,
            data_directory: format!("{}/data", get_exec_dir()?),
            cookie_duration_minutes: 43200,
            login_deadline_minutes: Some(43200),
            visit_deadline_minutes: Some(21600),
            session_secret_key_path: format!("{}/secret.key", get_exec_dir()?),
            max_username_size: 10,
            min_username_size: 3,
            max_passwd_size: 256,
            min_passwd_size: 9,
        })
    }
}

pub async fn open<P: AsRef<Path>>(path: P) -> Result<(), String> {
    let mut file = File::open(path)
        .await
        .map_err(|e| format!("Failed to open config file: {}", e))?;
    let mut config = String::new();
    file.read_to_string(&mut config)
        .await
        .map_err(|e| format!("Failed to read config file: {}", e))?;
    CONFIG
        .set(serde_yaml::from_str(&config).map_err(|e| format!("Failed to read config: {}", e))?)
        .expect("Config has already been opened");
    Ok(())
}

pub async fn write_default() -> Result<(), String> {
    let mut path = current_exe().map_err(|e| format!("Failed to get executable's path: {}", e))?;
    path.pop();
    path.push("default.yaml");
    let mut file = File::create(path)
        .await
        .map_err(|e| format!("Failed to create config file: {}", e))?;
    let default = Config::default()?;
    let default = serde_yaml::to_string(&default)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    file.write_all(&mut default.as_bytes())
        .await
        .map_err(|e| format!("Failed to write config: {}", e))?;
    Ok(())
}

pub fn make_url(url: &str) -> String {
    let prefix = config!(url_prefix);
    if prefix.is_empty() {
        url.into()
    } else {
        format!("/{}{}", prefix, url)
    }
}
