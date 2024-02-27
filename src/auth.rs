pub mod database;
pub mod error;
use crate::config;
use argon2::{
    password_hash::{
        errors, rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
    },
    Argon2,
};
use async_sqlite::Pool;
use error::{AuthError, DBError};
use std::io::{self, Write};

fn check_validity(username: &String, password: &Vec<u8>) -> Result<(), AuthError> {
    let user_len = username.len();
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
pub async fn check_credentials(
    pool: &Pool,
    username: &String,
    password: &Vec<u8>,
) -> Result<(), AuthError> {
    check_validity(username, password)?;
    let hash = database::get_user(pool, username.clone())
        .await
        .map_err(|e| AuthError::InternalError(e.to_string()))?
        .pass_hash;
    let parsed_hash = PasswordHash::new(&hash)
        .map_err(|e| AuthError::InternalError(format!("Failed to parse password hash: {}", e)))?;
    match Argon2::default().verify_password(password, &parsed_hash) {
        Ok(_) => Ok(()),
        Err(err) => match err {
            errors::Error::Password => Err(AuthError::InvalidCredentials),
            _ => Err(AuthError::InternalError(format!(
                "Failed to verify password: {}",
                err
            ))),
        },
    }
}

/// Adds a new user. Fails if username already exists
pub async fn add_user(pool: &Pool, username: String, password: &Vec<u8>) -> Result<(), AuthError> {
    check_validity(&username, password)?;
    let passwd_hash = {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        argon2
            .hash_password(password, &salt)
            .map_err(|e| AuthError::InternalError(format!("Failed to hash password: {}", e)))?
            .to_string()
    };
    if let Err(err) = database::add_user(pool, username, passwd_hash, false).await {
        match err {
            DBError::UserExists => Err(AuthError::InvalidRegCredentials),
            _ => Err(AuthError::InternalError(err.to_string())),
        }
    } else {
        Ok(())
    }
}

pub async fn cli_create_user() -> Result<(), DBError> {
    let pool = database::init_db().await?;
    let mut user = String::new();
    print!("User: ");
    io::stdout().flush().unwrap();
    io::stdin()
        .read_line(&mut user)
        .expect("Failed to read user");
    let user = user.trim().to_string();
    let mut password = rpassword::prompt_password("Password: ")
        .expect("Failed to read password")
        .into_bytes();
    add_user(&pool, user, &password).await?;
    password.zeroize();
    Ok(())
}
/*pub async fn delete(username: String) -> Result<(), AuthError> {
    Ok(())
}*/
