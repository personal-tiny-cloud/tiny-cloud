use crate::auth::DBError;
use crate::config;
use async_sqlite::{
    rusqlite::{self, named_params, ErrorCode, OptionalExtension},
    Error, JournalMode, Pool, PoolBuilder,
};
use rand::{distributions::Alphanumeric, Rng};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Types

#[non_exhaustive]
pub struct User {
    pub id: i64,
    pub username: String,
    pub pass_hash: String,
    #[cfg(feature = "totp-auth")]
    pub totp_secret: String,
    pub is_admin: bool,
}

#[non_exhaustive]
pub struct Token {
    pub id: i64,
    pub token: String,
    pub expire_date: i64,
}

/// Tables

#[cfg(not(feature = "totp-auth"))]
const USERS_TABLE: &str = "
CREATE TABLE IF NOT EXISTS users (
    id          INTEGER PRIMARY KEY,
    username    TEXT    NOT NULL,
    pass_hash   TEXT    NOT NULL,
    is_admin    INTEGER DEFAULT 0,
    UNIQUE(username)
);";

#[cfg(feature = "totp-auth")]
const USERS_TABLE: &str = "
CREATE TABLE IF NOT EXISTS users (
    id          INTEGER PRIMARY KEY,
    username    TEXT    NOT NULL,
    pass_hash   TEXT    NOT NULL,
    totp_secret TEXT    NOT NULL,
    is_admin    INTEGER DEFAULT 0,
    UNIQUE(username)
);";

const TOKEN_TABLE: &str = "
CREATE TABLE IF NOT EXISTS tokens (
    id          INTEGER PRIMARY KEY,
    token       TEXT    NOT NULL,
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

/// Functions

/// Connects to sqlite database and returns a pool.
/// Sets it to Wal mode by default, which is better for concurrency.
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

/// Adds a new user to the database, fails if it already exists.
/// If TOTP feature is enabled, it requires the totp-secret to be inserted
pub async fn add_user(
    pool: &Pool,
    username: String,
    pass_hash: String,
    #[cfg(feature = "totp-auth")] totp_secret: String,
    is_admin: bool,
) -> Result<(), DBError> {
    //if row_exists(pool, "users".into(), "username".into(), username.clone()).await? {
    //    return Err(DBError::UserExists);
    //}
    pool.conn(move |conn| {
        conn.execute(
            INSERT_USER,
            #[cfg(not(feature = "totp-auth"))]
            named_params! {
                ":username": username,
                ":pass_hash": pass_hash,
                ":is_admin": is_admin,
            },
            #[cfg(feature = "totp-auth")]
            named_params! {
                ":username": username,
                ":pass_hash": pass_hash,
                ":totp_secret": totp_secret,
                ":is_admin": is_admin,
            },
        )
    })
    .await
    .map_err(|e| {
        if let Error::Rusqlite(ref err) = e {
            if let rusqlite::Error::SqliteFailure(err, _) = err {
                if err.code == ErrorCode::ConstraintViolation {
                    return DBError::UserExists;
                }
            }
        }
        DBError::ExecError(format!("Failed to insert user: {e}"))
    })?;
    Ok(())
}

#[cfg(not(feature = "totp-auth"))]
fn get_user_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<User> {
    Ok(User {
        id: row.get(0)?,
        username: row.get(1)?,
        pass_hash: row.get(2)?,
        is_admin: row.get(3)?,
    })
}

#[cfg(feature = "totp-auth")]
fn get_user_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<User> {
    Ok(User {
        id: row.get(0)?,
        username: row.get(1)?,
        pass_hash: row.get(2)?,
        totp_secret: row.get(3)?,
        is_admin: row.get(4)?,
    })
}

/// Returns a user. User contains the TOTP secret depending on wether or not
/// the "totp-auth" feature is enabled.
pub async fn get_user(pool: &Pool, username: String) -> Result<Option<User>, DBError> {
    Ok(pool
        .conn(|conn| {
            conn.query_row(
                "SELECT * FROM users WHERE username=?1",
                [username],
                get_user_row,
            )
            .optional()
        })
        .await
        .map_err(|e| DBError::ExecError(format!("Failed to get user: {e}")))?)
}

/// Deletes a user from database
pub async fn delete_user(pool: &Pool, username: String) -> Result<(), DBError> {
    pool.conn(move |conn| conn.execute("DELETE FROM users WHERE username=?1", [username]))
        .await
        .map_err(|e| DBError::ExecError(format!("Failed to delete user: {e}")))?;
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
        .map_err(|e| DBError::ExecError(format!("Failed to create token: {e}")))?;
        Ok(_token)
    } else {
        Err(DBError::NotEnabled("registration tokens".into()))
    }
}

/// Gets token's data (id and expire date) if it exists
pub async fn get_token(pool: &Pool, token: String) -> Result<Option<Token>, DBError> {
    Ok(pool
        .conn(|conn| {
            conn.query_row("SELECT * FROM tokens WHERE token=?1", [token], |row| {
                Ok(Token {
                    id: row.get(0)?,
                    token: row.get(1)?,
                    expire_date: row.get(2)?,
                })
            })
            .optional()
        })
        .await
        .map_err(|e| DBError::ExecError(format!("Failed to get token: {}", e)))?)
}

/// Gets all saved tokens
pub async fn get_all_tokens(pool: &Pool) -> Result<Vec<Token>, DBError> {
    Ok(pool
        .conn(|conn| {
            let mut stmt = conn.prepare("SELECT * FROM tokens")?;
            let rows = stmt.query_map([], |row| {
                Ok(Token {
                    id: row.get(0)?,
                    token: row.get(1)?,
                    expire_date: row.get(2)?,
                })
            })?;
            rows.collect()
        })
        .await
        .map_err(|e| DBError::ExecError(format!("Failed to get tokens: {}", e)))?)
}

/// Removes all expired tokens
pub async fn remove_expired_tokens(pool: &Pool) -> Result<(), DBError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| DBError::TimeFailure("System clock may have gone backwards".into()))?
        .as_secs();
    pool.conn(move |conn| conn.execute("DELETE FROM tokens WHERE expire_date < ?1", [now]))
        .await
        .map_err(|e| DBError::ExecError(format!("Failed to remove expired tokens: {}", e)))?;
    Ok(())
}
