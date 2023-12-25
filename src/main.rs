mod api;
mod config;
//mod encryption;
mod auth;
mod plugins;
mod web_ui;
#[macro_use]
mod macros;
use actix_identity::IdentityMiddleware;
use actix_session::{config::PersistentSession, storage::CookieSessionStore, SessionMiddleware};
use actix_web::{
    cookie::{time::Duration, Key, SameSite},
    middleware, web, App, HttpServer,
};
use anyhow::{Context, Result};
use clap::Parser;
use std::{
    env,
    io::{self, Write},
};
use tokio::fs;
use zeroize::{Zeroize, Zeroizing};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path to the configuration file
    #[clap(short, long, value_parser, default_value = "config.yaml")]
    config: String,
    /// Writes the default configuration to default.yaml and exits
    #[clap(long = "write-default")]
    write_default: bool,
    /// Creates a new user and exits
    #[clap(long = "create-user")]
    create_user: bool,
}

async fn run() -> Result<()> {
    let args = Args::parse();

    if args.write_default {
        config::write_default()
            .await
            .context("Couldn't write default config")?;
        return Ok(());
    }

    config::open(args.config)
        .await
        .context("Couldn't open config")?;

    auth::init_db().await?;

    if args.create_user {
        let mut user = String::new();
        print!("User: ");
        io::stdout().flush().unwrap();
        io::stdin()
            .read_line(&mut user)
            .context("Failed to read user")?;
        let user = user.trim().to_string();
        let mut password = rpassword::prompt_password("Password: ")
            .context("Failed to read password")?
            .into_bytes();
        auth::set_user(user, &password)
            .await
            .map_err(|e| anyhow::format_err!("{}", e))?;
        password.zeroize();
        return Ok(());
    }

    let server = {
        let secret_key = Zeroizing::new(
            fs::read(config!(session_secret_key_path))
                .await
                .context("Couldn't read secret key file")?,
        );
        if secret_key.len() < 64 {
            return Err(anyhow::format_err!(
                "Session secret key must be 64 bytes long"
            ));
        }
        let secret_key = Key::from(&secret_key[..64]);

        let server = HttpServer::new(move || {
            App::new()
                .wrap(middleware::Logger::default())
                .wrap(middleware::NormalizePath::trim())
                .wrap(
                    IdentityMiddleware::builder()
                        .login_deadline(
                            config!(login_deadline_minutes)
                                .map(|d| std::time::Duration::from_secs(d * 60)),
                        )
                        .visit_deadline(
                            config!(visit_deadline_minutes)
                                .map(|d| std::time::Duration::from_secs(d * 60)),
                        )
                        .build(),
                )
                .wrap({
                    let session_middleware = SessionMiddleware::builder(
                        CookieSessionStore::default(),
                        secret_key.clone(),
                    )
                    .cookie_name("auth".to_owned())
                    .cookie_http_only(true)
                    .cookie_same_site(SameSite::Strict)
                    .session_lifecycle(
                        PersistentSession::default().session_ttl(Duration::minutes(
                            (*config!(cookie_duration_minutes)).into(),
                        )),
                    );
                    if cfg!(debug_assertions) {
                        session_middleware.cookie_secure(false).build()
                    } else {
                        session_middleware.build()
                    }
                })
                .service(web::redirect("/", "/tcloud/ui"))
                .service(
                    web::scope("/tcloud")
                        .service(
                            web::scope("/api")
                                .service(api::info)
                                .route("/app/{name}", web::get().to(api::plugin_handler))
                                .route("/app/{name}", web::post().to(api::plugin_handler))
                                .route("/app/{name}", web::put().to(api::plugin_handler))
                                .route("/app/{name}", web::delete().to(api::plugin_handler))
                                .route("/app/{name}", web::patch().to(api::plugin_handler))
                                .service(
                                    web::scope("/auth")
                                        .service(api::login)
                                        .service(api::logout)
                                        .service(api::delete),
                                ),
                        )
                        .service(web::scope("/ui").service(web_ui::root)),
                )
        });

        let server = if let Some(tls) = config!(tls) {
            server
                .bind_openssl(
                    format!("{}:{}", config!(server.host), config!(server.port)),
                    config::get_openssl_config(tls)?,
                )
                .context("Couldn't bind server with TLS")?
        } else {
            server
                .bind(format!("{}:{}", config!(server.host), config!(server.port)))
                .context("Couldn't bind server")?
        };
        server.workers(*config!(server.workers))
    };

    plugins::init()?;

    log::info!("Starting server...");
    server.run().await?;
    Ok(())
}

#[actix_web::main]
async fn main() {
    // Defaults to info if no RUST_LOG variable is set
    if let Err(_) = env::var("RUST_LOG") {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    if let Err(err) = run().await {
        log::error!("{}", err);
    }
}
