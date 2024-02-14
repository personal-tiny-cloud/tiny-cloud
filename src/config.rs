use anyhow::{Context, Result};
use openssl::ssl::{SslAcceptor, SslAcceptorBuilder, SslFiletype, SslMethod};
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
pub struct Tls {
    pub privkey_path: String,
    pub cert_path: String,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Registration {
    pub token: bool,
    pub token_duration_seconds: usize,
    pub max_accounts: usize,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server_name: String,
    pub description: String,
    pub server: Server,
    pub tls: Option<Tls>,
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

fn get_exec_dir() -> Result<String> {
    let mut path = current_exe().context("Failed to get executable's path")?;
    path.pop();
    Ok(path
        .to_str()
        .context("Failed to get executable's path")?
        .to_string())
}

impl Config {
    pub fn default() -> Result<Self> {
        Ok(Self {
            description: env!("CARGO_PKG_DESCRIPTION").to_string(),
            server_name: "Tiny Cloud".into(),
            server: Server {
                host: "127.0.0.1".into(),
                port: 80,
                workers: num_cpus::get(),
            },
            tls: None,
            registration: Some(Registration {
                token: true,
                token_duration_seconds: 3600,
                max_accounts: 50,
            }),
            max_account_storage_bytes: 10485750,
            data_directory: format!("{}/data", get_exec_dir()?),
            cookie_duration_minutes: 43200,
            login_deadline_minutes: Some(43200),
            visit_deadline_minutes: Some(21600),
            session_secret_key_path: format!("{}/secret.key", get_exec_dir()?),
            max_username_size: 20,
            min_username_size: 3,
            max_passwd_size: 256,
            min_passwd_size: 9,
        })
    }
}

pub async fn open<P: AsRef<Path>>(path: P) -> Result<()> {
    let mut file = File::open(path)
        .await
        .context("Failed to open config file")?;
    let mut config = String::new();
    file.read_to_string(&mut config)
        .await
        .context("Failed to read config file")?;
    CONFIG
        .set(serde_yaml::from_str(&config).context("Failed to parse config")?)
        .expect("Config has already been opened");
    Ok(())
}

pub async fn write_default() -> Result<()> {
    let mut path = current_exe().context("Failed to get executable's path")?;
    path.pop();
    path.push("default.yaml");
    let mut file = File::create(path)
        .await
        .context("Failed to create config file")?;
    let default = Config::default()?;
    let default = serde_yaml::to_string(&default).context("Failed to serialize config")?;
    file.write_all(&mut default.as_bytes())
        .await
        .context("Failed to write config")?;
    Ok(())
}

pub fn get_openssl_config(tls: &Tls) -> Result<SslAcceptorBuilder> {
    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls())
        .context("Failed to start openssl acceptor")?;
    builder
        .set_private_key_file(&tls.privkey_path, SslFiletype::PEM)
        .context("Failed to set private key file, is the path correct?")?;
    builder
        .set_certificate_chain_file(&tls.cert_path)
        .context("Failed to set certificate file, is the path correct?")?;
    Ok(builder)
}
