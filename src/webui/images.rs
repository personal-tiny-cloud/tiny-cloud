use actix_web::HttpResponse;

use crate::image;

const FAVICON: &'static [u8] = image!("favicon.ico");
const LOGO: &'static [u8] = image!("tiny-cloud-logo-256.png");

pub async fn favicon() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("image/x-icon")
        .body(FAVICON)
}

pub async fn logo() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("image/png")
        .body(LOGO)
}

