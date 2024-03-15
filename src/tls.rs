use crate::config::Tls;
#[cfg(any(feature = "openssl", feature = "openssl-bundled"))]
use openssl::ssl::{SslAcceptor, SslAcceptorBuilder, SslFiletype, SslMethod};
#[cfg(feature = "rustls")]
use rustls::{Certificate, PrivateKey, ServerConfig};
#[cfg(feature = "rustls")]
use rustls_pemfile::{certs, pkcs8_private_keys};

mutually_exclusive_features::exactly_one_of!("openssl", "openssl-bundled", "rustls");

#[cfg(any(feature = "openssl", feature = "openssl-bundled"))]
pub fn get_openssl_config(tls: &Tls) -> Result<SslAcceptorBuilder, String> {
    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls())
        .map_err(|e| format!("Failed to start openssl acceptor: {e}"))?;
    builder
        .set_private_key_file(&tls.privkey_path, SslFiletype::PEM)
        .map_err(|e| format!("Failed to get private key file: {e}"))?;
    builder
        .set_certificate_chain_file(&tls.cert_path)
        .map_err(|e| format!("Failed to get certificate file: {e}"))?;
    Ok(builder)
}

#[cfg(feature = "rustls")]
pub fn get_rustls_config(tls: &Tls) -> Result<rustls::ServerConfig, String> {
    // init server config builder with safe defaults
    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth();

    // load TLS key/cert files
    let cert_file = &mut BufReader::new(
        File::open(tls.cert_path).map_err(|e| format!("Failed to open certificate file: {e}"))?,
    );
    let key_file = &mut BufReader::new(
        File::open(tls.privkey_path)
            .map_err(|e| format!("Failed to open private key file: {e}"))?,
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
        return Err("Could not locate PKCS 8 private keys.".into());
    }

    Ok(config
        .with_single_cert(cert_chain, keys.remove(0))
        .map_err(|e| format!("Failed to parse certificate and key: {e}"))?)
}
