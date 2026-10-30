//! The rustls provider of Redis TLS connections.
//!
//! The Redis client builds its TLS configuration from the rustls process
//! default. When none is installed and an application's graph enables both
//! ring and aws-lc-rs, rustls cannot choose one and panics. Before a TLS
//! connection this installs aws-lc-rs, the framework's rustls provider, unless
//! the application already installed a provider.

/// Installs aws-lc-rs as the rustls process default before a `rediss://`
/// connection; plaintext connections and an installed provider are left alone.
pub(crate) fn install_provider_for(info: &redis::ConnectionInfo) {
    if matches!(info.addr(), redis::ConnectionAddr::TcpTls { .. })
        && rustls::crypto::CryptoProvider::get_default().is_none()
    {
        // Another thread may install first; its provider stays in force.
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    }
}

#[cfg(test)]
#[path = "redis_tls_tests.rs"]
mod tests;
