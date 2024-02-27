use crate::auth;
use crate::config;
use actix_identity::error::GetIdentityError;
use actix_identity::Identity;
use actix_web::{get, post, web, HttpMessage, HttpRequest, HttpResponse, Responder};
use serde::Deserialize;
use zeroize::Zeroizing;

#[derive(Deserialize)]
pub struct Login {
    user: String,
    password: String,
}

#[derive(Deserialize)]
pub struct Register {
    user: String,
    password: String,
    token: String,
}

/// Registers new user and starts a new session
#[post("/register")]
pub async fn register(req: HttpRequest, credentials: web::Json<Register>) -> impl Responder {
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
    if let Err(err) = auth::database::delete_user(username).await {
        handle_error!(err);
    } else {
        HttpResponse::Ok().body("")
    }
}
