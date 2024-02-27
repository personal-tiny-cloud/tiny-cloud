use thiserror::Error;

#[derive(Error, Debug)]
pub enum DBError {
    #[error("IO Error: `{0}`")]
    IOError(String),
    #[error("Execution of SQLite command failed: `{0}`")]
    ExecError(String),
    #[error("User already exists")]
    UserExists,
    #[error("Feature `{0}` is not enabled")]
    NotEnabled(String),
    #[error("Time failure: {0}")]
    TimeFailure(String),
}

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("An internal server error occurred")]
    InternalError(String),
    #[error("Bad credentials were given: {0}")]
    BadCredentials(String),
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Invalid registration credentials")]
    InvalidRegCredentials,
}

impl AuthError {
    pub fn http_code(&self) -> u16 {
        match self {
            Self::BadCredentials(_) => 400,
            Self::InvalidCredentials => 401,
            Self::InvalidRegCredentials => 401,
            Self::InternalError(_) => 500,
        }
    }
}
