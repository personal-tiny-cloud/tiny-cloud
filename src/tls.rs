use crate::config::Tls;
use anyhow::{Context, Result};
#[cfg(any(feature = "openssl", feature = "openssl-bundled"))]
use openssl::ssl::{SslAcceptor, SslAcceptorBuilder, SslFiletype, SslMethod};
#[cfg(feature = "rustls")]
use rustls::{Certificate, PrivateKey, ServerConfig};
#[cfg(feature = "rustls")]
use rustls_pemfile::{certs, pkcs8_private_keys};

mutually_exclusive_features::exactly_one_of!("openssl", "openssl-bundled", "rustls");

#[cfg(any(feature = "openssl", feature = "openssl-bundled"))]
pub fn get_openssl_config(tls: &Tls) -> Result<SslAcceptorBuilder> {
    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls())
        .context("Failed to start openssl acceptor")?;
    builder
        .set_private_key_file(&tls.privkey_path, SslFiletype::PEM)
        .context("Failed to get private key file, is the path correct?")?;
    builder
        .set_certificate_chain_file(&tls.cert_path)
        .context("Failed to get certificate file, is the path correct?")?;
    Ok(builder)
}

#[cfg(feature = "rustls")]
pub fn get_rustls_config(tls: &Tls) -> Result<rustls::ServerConfig> {
    // init server config builder with safe defaults
    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth();

    // load TLS key/cert files
    let cert_file = &mut BufReader::new(
        File::open(tls.cert_path)
            .context("Failed to open certificate file, is the path correct?")?,
    );
    let key_file = &mut BufReader::new(
        File::open(tls.privkey_path)
            .context("Failed to open private key file, is the path correct?")?,
    );

    // convert files to key/cert objects
    let cert_chain = certs(cert_file)
        .context("Failed to parse certificate")?
        .into_iter()
        .map(Certificate)
        .collect();
    let mut keys: Vec<PrivateKey> = pkcs8_private_keys(key_file)
        .context("Failed to parse private key")?
        .into_iter()
        .map(PrivateKey)
        .collect();

    // exit if no keys could be parsed
    if keys.is_empty() {
        return Err(anyhow::anyhow!("Could not locate PKCS 8 private keys."));
    }

    Ok(config
        .with_single_cert(cert_chain, keys.remove(0))
        .context("Failed to parse certificate and key")?)
}
