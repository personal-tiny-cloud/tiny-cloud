use actix_identity::Identity;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use include_dir::{include_dir, Dir};

/// HTML files
static HTML_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/html");

fn get_html(html: &str) -> String {
    HTML_DIR
        .get_file(html)
        .expect(&format!("Couldn't find {} file", html))
        .contents_utf8()
        .expect(&format!("Invalid UTF-8 file: {}", html))
        .to_owned()
}

#[get("")]
pub async fn root(user: Option<Identity>) -> impl Responder {
    if let Some(user) = user {
        let username = user.id().unwrap();
        HttpResponse::Ok().body(format!("Hi {}", username))
    } else {
        HttpResponse::Ok().body(get_html("login.html"))
    }
}
