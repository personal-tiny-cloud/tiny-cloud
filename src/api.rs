use crate::auth;
use crate::plugins::PLUGINS;
use crate::*;
use actix_identity::error::GetIdentityError;
use actix_identity::Identity;
use actix_web::http::StatusCode;
use actix_web::{get, post, web, HttpMessage, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;
use std::sync::OnceLock;
use tcloud_library::error::PluginError;
use zeroize::Zeroizing;

macro_rules! handle_error {
    ($err:expr) => {{
        match $err {
            auth::AuthError::InternalError(ref err) => {
                log::error!("An internal error occurred: {}", err);
                return HttpResponse::InternalServerError().body($err.to_string());
            }
            auth::AuthError::BadCredentials(_) => {
                return HttpResponse::BadRequest().body($err.to_string());
            }
            auth::AuthError::InvalidCredentials => {
                return HttpResponse::Forbidden().body($err.to_string());
            }
        }
    }};
}

macro_rules! get_user {
    ($id:expr) => {{
        match $id {
            Ok(user) => user,
            Err(err) => match err {
                GetIdentityError::SessionExpiryError(_) => {
                    return HttpResponse::Forbidden().body("The session has expired, login again")
                }
                GetIdentityError::MissingIdentityError(_) => {
                    return HttpResponse::Forbidden().body("Invalid session, login again")
                }
                _ => {
                    log::error!(
                        "An error occurred while getting username from identity: {}",
                        err
                    );
                    return HttpResponse::InternalServerError()
                        .body("An internal server error occurred while authenticating");
                }
            },
        }
    }};
}

const INFO: OnceLock<String> = OnceLock::new();

/// Returns server info
#[get("/info")]
pub async fn info() -> impl Responder {
    HttpResponse::Ok().body(
        INFO.get_or_init(|| {
            json!({
                "name": config!(server_name),
                "version": env!("CARGO_PKG_VERSION"),
                "description": config!(description),
                "source": env!("CARGO_PKG_REPOSITORY")
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
                HttpResponse::InternalServerError().body(format!(
                    "An IO error occurred while using `{}` plugin",
                    plugin_name
                ))
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
                return HttpResponse::InternalServerError()
                    .body("Internal server error occurred while making session");
            }
            HttpResponse::Ok().body("")
        }
        Err(err) => handle_error!(err),
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
    let username = get_user!(user.id());
    user.logout();
    if let Err(err) = auth::delete(username).await {
        handle_error!(err);
    } else {
        HttpResponse::Ok().body("")
    }
}
