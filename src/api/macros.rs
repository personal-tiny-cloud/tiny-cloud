#[macro_export]
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
