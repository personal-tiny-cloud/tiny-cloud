use crate::{
    auth::{self, error::AuthError},
    utils::get_ip,
};
use actix_identity::error::GetIdentityError;
use actix_identity::Identity;
use actix_web::{
    dev::ConnectionInfo, get, post, web, HttpMessage, HttpRequest, HttpResponse, Responder,
};
use async_sqlite::Pool;
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
/*
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
}*/

/// Logins and starts a new session
#[post("/login")]
pub async fn login(
    req: HttpRequest,
    conn: ConnectionInfo,
    login: web::Json<Login>,
    pool: web::Data<Pool>,
) -> impl Responder {
    let login = login.into_inner();
    let pool = pool.into_inner();
    let password = Zeroizing::new(login.password.into_bytes());
    match auth::check(&pool, &login.user, &password).await {
        Ok(_) => {
            if let Err(err) = Identity::login(&req.extensions(), login.user.clone()) {
                log::error!("Failed to build Identity: {err}");
                AuthError::InternalError("Failed to build identity during login".into())
                    .to_response()
            } else {
                log::warn!("host [{}] logged in as `{}`", get_ip(&conn), login.user);
                HttpResponse::Ok().body("")
            }
        }
        Err(err) => {
            if let AuthError::InvalidCredentials = err {
                log::warn!(
                    "host [{}] tried to login as `{}`",
                    get_ip(&conn),
                    login.user
                );
            }
            err.to_response()
        }
    }
}

/// Logs out and ends current session
#[get("/logout")]
pub async fn logout(user: Identity) -> impl Responder {
    user.logout();
    HttpResponse::Ok()
}

// Deletes an user's own account
#[get("/delete")]
pub async fn delete(user: Identity, pool: web::Data<Pool>) -> impl Responder {
    let username = get_user!(user.id());
    let pool = pool.into_inner();
    user.logout();
    if let Err(err) = auth::delete_user(&pool, username).await {
        err.to_response()
    } else {
        HttpResponse::Ok().body("")
    }
}
