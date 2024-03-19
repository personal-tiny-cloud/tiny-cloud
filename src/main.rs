mod api;
mod auth;
mod config;
mod logging;
mod plugins;
#[cfg(not(feature = "no-tls"))]
mod tls;
mod utils;
mod webui;
#[macro_use]
mod macros;
use actix_identity::IdentityMiddleware;
use actix_session::{config::PersistentSession, storage::CookieSessionStore, SessionMiddleware};
use actix_web::{
    cookie::{time::Duration, Key, SameSite},
    middleware,
    web::{self, Data},
    App, HttpServer,
};
use async_sqlite::Pool;
use clap::Parser;
use tokio::fs;
use zeroize::Zeroizing;

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

async fn server(secret_key: Key, database: Pool) -> Result<(), String> {
    let database = Data::new(database);
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
                let session_middleware =
                    SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                        .cookie_name("auth".to_owned())
                        .cookie_http_only(true)
                        .cookie_same_site(SameSite::Strict)
                        .session_lifecycle(PersistentSession::default().session_ttl(
                            Duration::minutes((*config!(cookie_duration_minutes)).into()),
                        ));
                if cfg!(not(feature = "no-tls")) {
                    session_middleware.build()
                } else {
                    session_middleware.cookie_secure(false).build()
                }
            })
            .app_data(Data::clone(&database))
            .service(
                web::scope(&utils::make_url("/static"))
                    .route("/favicon.ico", web::get().to(webui::images::favicon))
                    .route("/logo.png", web::get().to(webui::images::logo)),
            )
            .service(web::scope(&utils::make_url("/ui")).service(webui::root))
            .service(
                web::scope(&utils::make_url("/api"))
                    .service(api::info)
                    .route("/app/{name}", web::get().to(api::plugins::handler))
                    .route("/app/{name}", web::post().to(api::plugins::handler))
                    .route("/app/{name}", web::put().to(api::plugins::handler))
                    .route("/app/{name}", web::delete().to(api::plugins::handler))
                    .route("/app/{name}", web::patch().to(api::plugins::handler))
                    .service(
                        web::scope("/auth")
                            .service(api::auth::login)
                            .service(api::auth::logout)
                            .service(api::auth::delete),
                    ),
            )
    });

    // Setting TLS
    let server = {
        #[cfg(feature = "openssl")]
        {
            server
                .bind_openssl(
                    format!("{}:{}", config!(server.host), config!(server.port)),
                    tls::get_openssl_config(config!(tls))?,
                )
                .map_err(|e| format!("Failed to bind server with TLS (openssl): {e}"))?
        }

        #[cfg(feature = "rustls")]
        {
            server
                .bind_rustls_0_22(
                    format!("{}:{}", config!(server.host), config!(server.port)),
                    tls::get_rustls_config(config!(tls))?,
                )
                .map_err(|e| format!("Failed to bind server with TLS (rustls): {e}"))?
        }

        #[cfg(feature = "no-tls")]
        {
            server
                .bind(format!("{}:{}", config!(server.host), config!(server.port)))
                .map_err(|e| format!("Failed to bind server: {e}"))?
        }
    };

    plugins::init();

    log::info!("Starting server...");
    server
        .workers(*config!(server.workers))
        .run()
        .await
        .map_err(|e| format!("Error while running: {e}"))?;
    Ok(())
}

#[actix_web::main]
async fn main() {
    let args = Args::parse();

    if args.write_default {
        if let Err(e) = config::write_default().await {
            eprintln!("{e}");
        }
        return;
    }

    if let Err(e) = config::open(args.config).await {
        eprintln!("{e}");
        return;
    }

    if args.create_user {
        if let Err(e) = auth::cli::create_user().await {
            eprintln!("Failed to create user: {e}");
        }
        return;
    }

    if let Err(e) = logging::init_logging() {
        eprintln!("Failed to initialize logging: {e}");
        return;
    }

    let secret_key = {
        let path = config!(session_secret_key_path);
        match fs::read(path).await {
            Ok(b) => Zeroizing::new(b),
            Err(e) => {
                eprintln!("Failed to read secret key file `{path}`: {e}");
                return;
            }
        }
    };
    if secret_key.len() < 64 {
        eprintln!("Session secret key must be 64 bytes long");
        return;
    }
    let secret_key = Key::from(&secret_key[..64]);

    let database = match auth::database::init_db().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Failed to initialize Database: {e}");
            return;
        }
    };

    if let Err(e) = server(secret_key, database).await {
        log::error!("Server crashed: {e}");
    }
}
