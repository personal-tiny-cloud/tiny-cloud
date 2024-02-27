use crate::auth::DBError;
use crate::config;
use async_sqlite::{
    rusqlite::{self, named_params, OptionalExtension},
    JournalMode, Pool, PoolBuilder,
};
use rand::{distributions::Alphanumeric, Rng};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(not(feature = "totp-auth"))]
#[non_exhaustive]
pub struct User {
    pub username: String,
    pub pass_hash: String,
    pub is_admin: bool,
}

#[cfg(feature = "totp-auth")]
#[non_exhaustive]
struct User {
    pub username: String,
    pub pass_hash: String,
    pub totp_secret: String,
    pub is_admin: bool,
}

/// Tables
#[cfg(not(feature = "totp-auth"))]
const USERS_TABLE: &str = "
CREATE TABLE IF NOT EXISTS users (
    username TEXT NOT NULL,
    pass_hash TEXT NOT NULL,
    is_admin INTEGER DEFAULT 0,
    UNIQUE(username)
);";

#[cfg(feature = "totp-auth")]
const USERS_TABLE: &str = "
CREATE TABLE IF NOT EXISTS users (
    username TEXT NOT NULL,
    pass_hash TEXT NOT NULL,
    totp_secret TEXT NOT NULL,
    is_admin INTEGER DEFAULT 0,
    UNIQUE(username)
);";

const TOKEN_TABLE: &str = "
CREATE TABLE IF NOT EXISTS tokens (
    id INTEGER PRIMARY KEY,
    token TEXT NOT NULL,
    expire_date INTEGER NOT NULL,
    UNIQUE(token)
);";

/// Insertions
#[cfg(not(feature = "totp-auth"))]
const INSERT_USER: &str =
    "INSERT INTO users (username, pass_hash, is_admin) VALUES (:username, :pass_hash, :is_admin)";

#[cfg(feature = "totp-auth")]
const INSERT_USER: &str = "INSERT INTO users (username, pass_hash, totp-secret, is_admin) VALUES (:username, :pass_hash, :totp-secret, :is_admin)";

const INSERT_TOKEN: &str = "INSERT INTO tokens (token, expire_date) VALUES (:token, :expire_date)";

fn get_tables() -> String {
    if let Some(_) = config!(registration) {
        format!("BEGIN;\n{}\n{}\nCOMMIT;", USERS_TABLE, TOKEN_TABLE)
    } else {
        format!("BEGIN;\n{}\nCOMMIT;", USERS_TABLE)
    }
}

pub async fn init_db() -> Result<Pool, DBError> {
    let mut data_path = PathBuf::from(config!(data_directory));
    data_path.push("auth.db");
    let pool = PoolBuilder::new()
        .journal_mode(JournalMode::Wal)
        .path(data_path)
        .open()
        .await
        .map_err(|e| DBError::IOError(e.to_string()))?;
    pool.conn(|conn| conn.execute_batch(&get_tables()))
        .await
        .map_err(|e| DBError::ExecError(format!("Failed to initialize tables: {}", e)))?;
    Ok(pool)
}

async fn row_exists(
    pool: &Pool,
    table: String,
    value_name: String,
    value: &String,
) -> Result<bool, DBError> {
    let value = value.clone();
    let user: Option<String> = pool
        .conn(move |conn| {
            conn.query_row(
                "SELECT ?1 FROM ?2 WHERE ?1=?3",
                [value_name, table, value],
                |row| row.get(0),
            )
            .optional()
        })
        .await
        .map_err(|e| DBError::ExecError(format!("Failed to check if row exists: {}", e)))?;
    if let Some(_) = user {
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Adds a new user to the database, fails if it already exists
#[cfg(not(feature = "totp-auth"))]
pub async fn add_user(
    pool: &Pool,
    username: String,
    pass_hash: String,
    is_admin: bool,
) -> Result<(), DBError> {
    if row_exists(pool, "users".into(), "username".into(), &username).await? {
        return Err(DBError::UserExists);
    }
    pool.conn(move |conn| {
        conn.execute(
            INSERT_USER,
            named_params! {
                ":username": username,
                ":pass_hash": pass_hash,
                ":is_admin": is_admin,
            },
        )
    })
    .await
    .map_err(|e| DBError::ExecError(format!("Failed to insert user: {}", e)))?;
    Ok(())
}

#[cfg(feature = "totp-auth")]
pub async fn add_user(
    pool: &Pool,
    username: String,
    pass_hash: String,
    totp_secret: String,
    is_admin: bool,
) -> Result<(), DBError> {
    if row_exists(pool, "users".into(), "username".into(), &username).await? {
        return Err(DBError::UserExists);
    }
    pool.conn(move |conn| {
        conn.execute(
            INSERT_USER,
            named_params! {
                ":username": username,
                ":pass_hash": pass_hash,
                ":totp_secret": totp_secret,
                ":is_admin": is_admin,
            },
        )
    })
    .await
    .map_err(|e| DBError::ExecError(format!("Failed to insert user: {}", e)))?;
    Ok(())
}

#[cfg(not(feature = "totp-auth"))]
fn get_user_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<User> {
    Ok(User {
        username: row.get(0)?,
        pass_hash: row.get(1)?,
        is_admin: row.get(2)?,
    })
}

#[cfg(feature = "totp-auth")]
fn get_user_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<User> {
    Ok(User {
        username: row.get(0)?,
        pass_hash: row.get(1)?,
        totp_secret: row.get(2)?,
        is_admin: row.get(3)?,
    })
}

pub async fn get_user(pool: &Pool, username: String) -> Result<User, DBError> {
    Ok(pool
        .conn(|conn| {
            conn.query_row(
                "SELECT * FROM users WHERE username=?1",
                [username],
                get_user_row,
            )
        })
        .await
        .map_err(|e| DBError::ExecError(format!("Failed to get user: {}", e)))?)
}

pub async fn delete_user(pool: &Pool, username: String) -> Result<(), DBError> {
    pool.conn(move |conn| conn.execute("DELETE FROM users WHERE username=?1", [username]))
        .await
        .map_err(|e| DBError::ExecError(format!("Failed to delete user: {}", e)))?;
    Ok(())
}

/// Creates a token and adds it to the database
/// Optionally takes an `duration_secs` param which specifies the duration, if none
/// is given then the config's token_duration_seconds is used
pub async fn create_token(pool: &Pool, duration_secs: Option<u64>) -> Result<String, DBError> {
    if let Some(registration) = config!(registration) {
        let token: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(registration.token_size.into())
            .map(char::from)
            .collect();
        let _token = token.clone();
        let duration = if let Some(duration) = duration_secs {
            duration
        } else {
            registration.token_duration_seconds
        };
        let expire_date: u64 = SystemTime::now()
            .checked_add(Duration::new(duration, 0))
            .ok_or(DBError::TimeFailure(
                "Failed to calculate token's expire date".into(),
            ))?
            .duration_since(UNIX_EPOCH)
            .map_err(|_| DBError::TimeFailure("System clock may have gone backwards".into()))?
            .as_secs();
        pool.conn(move |conn| {
            conn.execute(
                INSERT_TOKEN,
                named_params! {
                    ":token": token,
                    ":expire_date": expire_date,
                },
            )
        })
        .await
        .map_err(|e| DBError::ExecError(format!("Failed to create token: {}", e)))?;
        Ok(_token)
    } else {
        Err(DBError::NotEnabled("registration tokens".into()))
    }
}
