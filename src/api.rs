use crate::auth;
use crate::plugins::PLUGINS;
use crate::*;
use actix_identity::Identity;
use actix_web::http::StatusCode;
use actix_web::{get, post, web, HttpMessage, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;
use std::sync::OnceLock;
use tcloud_library::error::PluginError;
use zeroize::Zeroizing;

/// API Errors returned to the client
#[derive(Serialize)]
#[serde(tag = "error", content = "message")]
enum ErrorTypes {
    /// Returned when credentials don't meet the necessary length or format requirements
    BadCredentials(String),
    /// Returned when the password is wrong
    BadPassword(String),
    /// Returned when the requested user is not found
    UserNotFound(String),
    /// Returned when an internal server error happens
    InternalServerError(String),
}

impl ErrorTypes {
    fn http_code(&self) -> u16 {
        match self {
            Self::BadCredentials(_) => 400,
            Self::BadPassword(_) => 401,
            Self::UserNotFound(_) => 404,
            Self::InternalServerError(_) => 500,
        }
    }
}

macro_rules! mkresponse {
    ($error:ident, $message:expr) => {{
        let error = ErrorTypes::$error($message.to_owned());
        HttpResponse::build(
            StatusCode::from_u16(error.http_code()).expect("An invalid HTTP code has been used"),
        )
        .body(serde_json::to_string(&error).expect("Error serialization failed"))
    }};
}

const INFO: OnceLock<String> = OnceLock::new();

/// Returns server info
#[get("/info")]
pub async fn info() -> impl Responder {
    HttpResponse::Ok().body(
        INFO.get_or_init(|| {
            json!({
                "version": env!("CARGO_PKG_VERSION"),
                "description": config!(description)
            })
            .to_string()
        })
        .to_owned(),
    )
}

/// Handles plugins
pub async fn plugin_handler(req: HttpRequest, path: web::Path<String>) -> HttpResponse {
    let plugin_name = path.into_inner();
    let plugins = PLUGINS.get().expect("Plugins haven't been initialized");
    let plugin = match plugins.get(&plugin_name) {
        Some(plugin) => plugin,
        None => {
            return HttpResponse::NotFound().body(format!("Plugin `{}` not found", plugin_name))
        }
    };
    let mut plugin = plugin.lock().await;
    match plugin
        .process_api_request("test".to_string(), &Path::new("./test"))
        .await
    {
        Ok(resp) => HttpResponse::Ok().body(resp),
        Err(err) => match err {
            PluginError::IOError(err) => {
                log::error!("Internal IO Error: {:?}", err);
                mkresponse!(
                    InternalServerError,
                    format!("An IO error occurred while using `{}` plugin", plugin_name)
                )
            }
            PluginError::InvalidRequest(resp) => HttpResponse::BadRequest().body(resp),
            PluginError::RequestFailed(code, resp) => {
                HttpResponse::build(StatusCode::from_u16(code).expect(&format!(
                    "Invalid HTTP Status Code from `{}` plugin",
                    plugin_name
                )))
                .body(resp)
            }
            _ => panic!(
                "Invalid error returned by `{}` plugin while handling request",
                plugin_name
            ),
        },
    }
}

macro_rules! handle_db_error {
    ($err:expr) => {{
        match $err {
            auth::DBError::IOError(err)
            | auth::DBError::HashingError(err)
            | auth::DBError::SerializationError(err) => {
                log::error!("An error occurred during login: {}", err);
                return mkresponse!(InternalServerError, "An internal error occurred");
            }
            auth::DBError::UserNotFound => {
                return mkresponse!(UserNotFound, "This user has not been found")
            }
            auth::DBError::BadCredentials(_) => {
                return mkresponse!(BadCredentials, format!("{}", $err))
            }
            auth::DBError::BadPassword => return mkresponse!(BadPassword, format!("{}", $err)),
        }
    }};
}

#[derive(Deserialize)]
pub struct Login {
    user: String,
    password: String,
}

#[derive(Deserialize)]
pub struct Register {
    user: String,
    password: String,
    token: Option<String>,
}

/// Registers new user and starts a new session
#[post("/register")]
pub async fn register(
    req: HttpRequest,
    credentials: web::Json<Register>,
    tokens: web::Data<auth::Tokens>,
) -> impl Responder {
    if let Some(registration) = config!(registration) {
        let credentials = credentials.into_inner();
        if registration.token {
        } else {
        }
        HttpResponse::Ok()
    } else {
        HttpResponse::NotFound()
    }
}

/// Logins and starts a new session
#[post("/login")]
pub async fn login(req: HttpRequest, login: web::Json<Login>) -> impl Responder {
    let login = login.into_inner();
    let password = Zeroizing::new(login.password.into_bytes());
    match auth::check_passwd(&login.user, &password).await {
        Ok(_) => {
            if let Err(err) = Identity::login(&req.extensions(), login.user) {
                log::error!("Failed to build Identity: {}", err);
                return mkresponse!(InternalServerError, "Failed to build session");
            }
            HttpResponse::Ok().body("")
        }
        Err(err) => handle_db_error!(err),
    }
}

/// Logs out and ends current session
#[get("/logout")]
pub async fn logout(user: Identity) -> impl Responder {
    user.logout();
    HttpResponse::Ok()
}

/// Deletes an user's own account
#[get("/delete")]
pub async fn delete(user: Identity) -> impl Responder {
    let username = match user.id() {
        Ok(user) => user,
        Err(err) => {
            log::error!("Failed to get user id: {}", err);
            return mkresponse!(InternalServerError, "An internal server error occurred");
        }
    };
    user.logout();
    if let Err(err) = auth::delete(username).await {
        handle_db_error!(err);
    } else {
        HttpResponse::Ok().body("")
    }
}
