pub mod cli;
pub mod database;
pub mod error;
mod hash;
use crate::config;
use async_sqlite::Pool;
use error::{AuthError, DBError};

fn check_validity(username: &String, password: &Vec<u8>) -> Result<(), AuthError> {
    let user_len = username.len();
    let passwd_len = password.len();
    let max_username_size = *config!(max_username_size) as usize;
    let min_username_size = *config!(min_username_size) as usize;
    let max_passwd_size = *config!(max_passwd_size) as usize;
    let min_passwd_size = *config!(min_passwd_size) as usize;
    if user_len > max_username_size || user_len < min_username_size {
        return Err(AuthError::BadCredentials(format!(
            "Accepted username size is between {min_username_size} and {max_username_size} characters",
        )));
    }
    if passwd_len > max_passwd_size || passwd_len < min_passwd_size {
        return Err(AuthError::BadCredentials(format!(
            "Accepted password length is between {min_passwd_size} and {max_passwd_size} bytes",
        )));
    }
    for c in username.chars() {
        if !c.is_alphanumeric() {
            return Err(AuthError::BadCredentials(
                "Username must be alphanumerical".into(),
            ));
        }
    }
    Ok(())
}

/// Checks a user's password
pub async fn check(pool: &Pool, username: &String, password: &Vec<u8>) -> Result<(), AuthError> {
    check_validity(username, password)?;
    let hash = database::get_user(pool, username.clone())
        .await
        .map_err(|e| AuthError::InternalError(e.to_string()))?
        .pass_hash;
    hash::verify(password, &hash)
}

/// Adds a new user. Fails if username already exists
pub async fn add_user(
    pool: &Pool,
    username: String,
    password: &Vec<u8>,
    is_admin: bool,
) -> Result<(), AuthError> {
    check_validity(&username, password)?;
    let passwd_hash = hash::create(password)?;
    if let Err(err) = database::add_user(pool, username, passwd_hash, is_admin).await {
        match err {
            DBError::UserExists => Err(AuthError::InvalidRegCredentials),
            _ => Err(AuthError::InternalError(err.to_string())),
        }
    } else {
        Ok(())
    }
}

pub async fn delete_user(pool: &Pool, username: String) -> Result<(), AuthError> {
    database::delete_user(&pool, username)
        .await
        .map_err(|e| AuthError::InternalError(e.to_string()))?;
    Ok(())
}
