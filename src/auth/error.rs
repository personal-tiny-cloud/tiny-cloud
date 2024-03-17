use actix_web::{http::StatusCode, HttpResponse};
use serde_json::json;
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
    InvalidRegCredentials(String),
}

impl AuthError {
    pub fn name(&self) -> String {
        match self {
            Self::BadCredentials(_) => "BadCredentials",
            Self::InvalidCredentials => "InvalidCredentials",
            Self::InvalidRegCredentials(_) => "InvalidRegCredentials",
            Self::InternalError(_) => "InternalError",
        }
        .into()
    }

    pub fn to_response(&self) -> HttpResponse {
        if let Self::InternalError(err) = self {
            log::error!("An internal server error occurred during authentication: {err}");
        }
        HttpResponse::build(
            StatusCode::from_u16(self.http_code())
                .expect("Invalid http code returned by http_code(). This is a bug"),
        )
        .body(
            json!({
                "error": self.name(),
                "msg": self.to_string()
            })
            .to_string(),
        )
    }

    pub fn http_code(&self) -> u16 {
        match self {
            Self::BadCredentials(_) => 400,
            Self::InvalidCredentials => 401,
            Self::InvalidRegCredentials(_) => 401,
            Self::InternalError(_) => 500,
        }
    }
}
