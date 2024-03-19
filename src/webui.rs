pub mod images;
mod home;
mod login;
#[macro_use]
mod macros;
use actix_identity::Identity;
use actix_web::{get, Result as AwResult};
use maud::Markup;

#[get("")]
pub async fn root(user: Option<Identity>) -> AwResult<Markup> {
    if let Some(user) = user {
        let username = user.id().unwrap();
        Ok(home::page(username))
    } else {
        Ok(login::page())
    }
}
