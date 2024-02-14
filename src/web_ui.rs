use actix_identity::Identity;
use actix_web::{get, Result as AwResult};
use maud::{html, Markup, PreEscaped, DOCTYPE};

use crate::{config, web_file};

#[get("")]
pub async fn root(user: Option<Identity>) -> AwResult<Markup> {
    if let Some(user) = user {
        let username = user.id().unwrap();
        Ok(html! {
            (DOCTYPE)
            html lang="en-US" {
                head {
                    title { "Main page" }
                    meta name="application-name" content=(config!(server_name));
                    meta charset="utf-8";
                    meta name="viewport" content="width=device-width, initial-scale=1.0";
                }
                body {
                    h1 { "Hi " (username) }
                }
            }
        })
    } else {
        Ok(html! {
            (DOCTYPE)
            html lang="en-US" {
                head {
                    title { "Login page" }
                    meta name="application-name" content=(config!(server_name));
                    meta charset="utf-8";
                    meta name="viewport" content="width=device-width, initial-scale=1.0";
                    script type="text/javascript" { (web_file!("login.js")) }
                    style { (web_file!("login.css")) }
                }
                body {
                   p { div id="title" { (config!(server_name)) } }
                   p { div id="version" { (env!("CARGO_PKG_VERSION")) } }
                   p { div id="description" { (config!(description)) } }
                   form id="login" name="login" {
                        br { label for="user" { "Username:" } }
                        br { input type="text" id="user" name="user"; }
                        br { label for="password" { "Password:" } }
                        br { input type="password" id="password" name="password"; }
                        input value="Login" type="submit";
                   }
                }
                div id="errormsg";
            }
        })
    }
}
