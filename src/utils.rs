use crate::config;
use actix_web::dev::ConnectionInfo;

/// Creates URL using the prefix specified in settings
pub fn make_url(url: &str) -> String {
    let prefix = config!(url_prefix);
    if prefix.is_empty() {
        url.into()
    } else {
        format!("/{}{}", prefix, url)
    }
}

/// Gets ip of connection's info from the most reliable source
/// depending on wether or not the server is behind a proxy
pub fn get_ip(conn: &ConnectionInfo) -> &str {
    if *config!(server.is_behind_proxy) {
        conn.realip_remote_addr()
    } else {
        conn.peer_addr()
    }
    .unwrap_or("unknown")
}
