#[macro_export]
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
