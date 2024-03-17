use crate::auth::{add_user, database, error::AuthError};
use std::io::{self, Write};
use zeroize::Zeroize;

pub async fn create_user() -> Result<(), String> {
    // Init DB
    let pool = database::init_db().await.map_err(|e| e.to_string())?;

    let mut input = String::new();

    // Gets user from CLI
    print!("User: ");
    io::stdout().flush().unwrap();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read user");
    let user = input.trim().to_string();

    // Gets password from CLI using a safe input
    let mut password = {
        let first = rpassword::prompt_password("Password: ")
            .expect("Failed to read password")
            .into_bytes();
        let mut second = rpassword::prompt_password("Confirm password: ")
            .expect("Confirm password")
            .into_bytes();
        if first != second {
            second.zeroize();
            return Err("Passwords do not match.".into());
        } else {
            second.zeroize();
            first
        }
    };

    // Make user admin?
    print!("Make user admin? [y/n] ");
    io::stdout().flush().unwrap();
    input.clear();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read admin request");
    let is_admin = input.trim().to_string().to_lowercase() == "y";

    // Add user to DB
    add_user(&pool, user.clone(), &password, is_admin)
        .await
        .map_err(|e| match e {
            AuthError::InvalidRegCredentials(ref err) => format!("{e}: {err}"),
            AuthError::InternalError(ref err) => format!("{e}: {err}"),
            _ => e.to_string(),
        })?;

    if is_admin {
        println!(
            "Successfully added admin {} with password length {}",
            user,
            password.len()
        );
    } else {
        println!(
            "Successfully added user {} with password length {}",
            user,
            password.len()
        );
    }

    password.zeroize();
    Ok(())
}
