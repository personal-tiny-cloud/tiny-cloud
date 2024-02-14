use crate::config;
use anyhow::{Context, Result};
use argon2::{
    password_hash::{
        errors, rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
    },
    Argon2,
};
use std::collections::HashMap;
use std::io;
use thiserror::Error;
use tokio::{
    fs::{self, File},
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{OnceCell, RwLock},
};

macro_rules! get_db {
    ($mode:ident) => {{
        DB.get().expect("DB not initialized").$mode().await
    }};
}

macro_rules! get_db_path {
    () => {{
        DB_PATH.get().expect("DB not initialized")
    }};
}

#[derive(Error, Debug)]
pub enum DBError {
    #[error("IO Error: `{0}`")]
    IOError(String),
    #[error("Serialization failed: `{0}`")]
    SerializationError(String),
    #[error("Password hashing failed: `{0}`")]
    HashingError(String),
}

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("An internal server error occurred")]
    InternalError(DBError),
    #[error("Bad credentials were given: {0}")]
    BadCredentials(String),
    #[error("Invalid credentials")]
    InvalidCredentials,
}

impl AuthError {
    fn http_code(&self) -> u16 {
        match self {
            Self::BadCredentials(_) => 400,
            Self::InvalidCredentials => 401,
            Self::InternalError(_) => 500,
        }
    }
}

/// Struct containing valid tokens which can be used to make new accounts.
/// String is the token, while usize is the expiration time (unix time).
/// All tokens will be removed when shutting down the server
pub enum Tokens {
    Enabled(RwLock<HashMap<String, usize>>),
    Disabled,
}

pub static DB: OnceCell<RwLock<HashMap<String, String>>> = OnceCell::const_new();
pub static DB_PATH: OnceCell<String> = OnceCell::const_new();

impl Default for Tokens {
    fn default() -> Self {
        if let Some(registration) = config!(registration) {
            if registration.token {
                return Self::Enabled(RwLock::new(HashMap::new()));
            }
        }
        Self::Disabled
    }
}

/// Initializes DB global variables.
/// Must be executed during server initialization.
pub async fn init_db() -> Result<()> {
    let db_path = format!("{}/users.json", config!(data_directory));
    match File::open(&db_path).await {
        Ok(mut file) => {
            let mut users = String::new();
            file.read_to_string(&mut users)
                .await
                .context("Failed to read users DB")?;
            let users = serde_json::from_str(&users).context("Failed to parse users DB")?;
            DB.set(RwLock::new(users))
                .expect("DB has already been initialized");
        }
        Err(err) => match err.kind() {
            io::ErrorKind::NotFound => DB
                .set(RwLock::new(HashMap::new()))
                .expect("DB has already been initialized"),
            _ => return Err(anyhow::format_err!("Failed to open users DB file: {}", err)),
        },
    }
    DB_PATH
        .set(db_path)
        .expect("DB has already been initialized");
    Ok(())
}

/// Dumps DB to file
async fn dump_db() -> Result<(), DBError> {
    let users = get_db!(read);
    let path = get_db_path!();
    let mut file = if fs::try_exists(&path)
        .await
        .map_err(|e| DBError::IOError(format!("{}", e)))?
    {
        fs::remove_file(&path)
            .await
            .map_err(|e| DBError::IOError(format!("{}", e)))?;
        File::create(&path)
            .await
            .map_err(|e| DBError::IOError(format!("{}", e)))?
    } else {
        File::create(&path)
            .await
            .map_err(|e| DBError::IOError(format!("{}", e)))?
    };
    let users = users.clone();
    let serialized: String =
        serde_json::to_string(&users).map_err(|e| DBError::SerializationError(format!("{}", e)))?;
    file.write_all(serialized.as_bytes())
        .await
        .map_err(|e| DBError::SerializationError(format!("{}", e)))?;
    Ok(())
}

fn check_validity(user: &String, password: &Vec<u8>) -> Result<(), AuthError> {
    let user_len = user.len();
    let passwd_len = password.len();
    let max_username_size = *config!(max_username_size) as usize;
    let min_username_size = *config!(min_username_size) as usize;
    let max_passwd_size = *config!(max_passwd_size) as usize;
    let min_passwd_size = *config!(min_passwd_size) as usize;
    if user_len > max_username_size || user_len < min_username_size {
        return Err(AuthError::BadCredentials(format!(
            "Accepted username size is between {} and {} characters",
            min_username_size, max_username_size
        )));
    }
    if passwd_len > max_passwd_size || passwd_len < min_passwd_size {
        return Err(AuthError::BadCredentials(format!(
            "Accepted password length is between {} and {} bytes",
            min_passwd_size, max_passwd_size
        )));
    }
    for c in user.chars() {
        if !c.is_alphanumeric() {
            return Err(AuthError::BadCredentials(format!(
                "Username must be alphanumerical"
            )));
        }
    }
    Ok(())
}

/// Checks a user's password
pub async fn check_passwd(user: &String, password: &Vec<u8>) -> Result<(), AuthError> {
    check_validity(user, password)?;
    let hash = {
        let users = get_db!(read);
        let user = match users.get(user) {
            Some(hash) => hash,
            None => return Err(AuthError::InvalidCredentials),
        };
        user.clone()
    };
    let parsed_hash = PasswordHash::new(&hash)
        .map_err(|e| AuthError::InternalError(DBError::HashingError(format!("{}", e))))?;
    match Argon2::default().verify_password(password, &parsed_hash) {
        Ok(_) => Ok(()),
        Err(err) => match err {
            errors::Error::Password => Err(AuthError::InvalidCredentials),
            _ => Err(AuthError::InternalError(DBError::HashingError(format!(
                "{}",
                err
            )))),
        },
    }
}

/// Sets a new user. If the user already exists, the password or admin status is changed.
pub async fn set_user(user: String, password: &Vec<u8>) -> Result<(), AuthError> {
    check_validity(&user, password)?;
    let passwd_hash = {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        argon2
            .hash_password(password, &salt)
            .map_err(|e| AuthError::InternalError(DBError::HashingError(format!("{}", e))))?
            .to_string()
    };
    {
        let mut users = get_db!(write);
        users.insert(user, passwd_hash);
    }
    dump_db().await.map_err(|e| AuthError::InternalError(e))?;
    Ok(())
}

pub async fn delete(user: String) -> Result<(), AuthError> {
    {
        let mut users = get_db!(write);
        users.remove(&user);
    }
    dump_db().await.map_err(|e| AuthError::InternalError(e))?;
    Ok(())
}
