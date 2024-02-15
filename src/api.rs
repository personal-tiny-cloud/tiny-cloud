#[macro_use]
mod macros;
pub mod auth;
pub mod plugins;
use crate::config;
use actix_web::{get, HttpResponse, Responder};
use serde_json::json;
use std::sync::OnceLock;

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
