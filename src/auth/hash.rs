use argon2::{
    password_hash::{
        errors, rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
    },
    Argon2,
};

use super::error::AuthError;

pub fn verify(password: &[u8], hash: &str) -> Result<(), AuthError> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| AuthError::InternalError(format!("Failed to parse password hash: {e}")))?;
    match Argon2::default().verify_password(password, &parsed_hash) {
        Ok(_) => Ok(()),
        Err(err) => match err {
            errors::Error::Password => Err(AuthError::InvalidCredentials),
            _ => Err(AuthError::InternalError(format!(
                "Failed to verify password: {err}"
            ))),
        },
    }
}

pub fn create(password: &[u8]) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password, &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| AuthError::InternalError(format!("Failed to hash password: {e}")))
}
