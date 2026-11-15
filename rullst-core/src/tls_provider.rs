//! The rustls process-wide crypto provider.
//!
//! aws-lc-rs is the rustls provider for every TLS connection Rullst makes:
//! outbound HTTPS, SQLx, Redis, SMTP and OTLP. Core does not depend on ring:
//! it hashes with RustCrypto `sha2` and encrypts `storage-multipart` recovery
//! records with `aes-gcm`. Clients that read the process default (Redis, and reqwest or
//! lettre once a default exists) would otherwise depend on initialisation
//! order, and rustls cannot pick a default when an application's graph
//! enables both providers.

/// Installs aws-lc-rs as the rustls process default unless a provider is
/// already installed.
///
/// An application that installed its own provider first keeps it: rustls
/// refuses a second installation, and that refusal is ignored.
#[cfg(feature = "tls")]
pub(crate) fn install_default_provider() {
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    }
}

/// Without `tls`, Core compiles no rustls: every feature that makes a TLS
/// connection enables it, and SQLite needs none.
#[cfg(not(feature = "tls"))]
pub(crate) fn install_default_provider() {}
