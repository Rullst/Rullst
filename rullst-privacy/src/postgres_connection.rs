//! Shared private PostgreSQL transport policy for privacy stores.

use sqlx::{
    ConnectOptions,
    postgres::{PgConnectOptions, PgSslMode},
};
use std::{net::IpAddr, str::FromStr};

#[derive(Debug)]
pub(crate) enum PgOptionsError {
    InvalidConfiguration,
}

pub(crate) fn connection_options(url: &str) -> Result<PgConnectOptions, PgOptionsError> {
    if url.len() > 8192 || !(url.starts_with("postgres://") || url.starts_with("postgresql://")) {
        return Err(PgOptionsError::InvalidConfiguration);
    }
    // SQLx logs unknown query parameter values. Reject them before invoking its
    // parser so a misspelled password/token option cannot enter ordinary logs.
    let parsed = url::Url::parse(url).map_err(|_| PgOptionsError::InvalidConfiguration)?;
    let mut keys = std::collections::BTreeSet::new();
    if parsed.fragment().is_some()
        || parsed.query_pairs().any(|(key, _)| {
            !matches!(
                key.as_ref(),
                "sslmode"
                    | "ssl-mode"
                    | "sslrootcert"
                    | "ssl-root-cert"
                    | "ssl-ca"
                    | "sslcert"
                    | "ssl-cert"
                    | "sslkey"
                    | "ssl-key"
                    | "host"
                    | "hostaddr"
                    | "port"
                    | "dbname"
                    | "user"
                    | "password"
                    | "application_name"
                    | "statement-cache-capacity"
            ) || !keys.insert(key.into_owned())
        })
    {
        return Err(PgOptionsError::InvalidConfiguration);
    }
    let mut options =
        PgConnectOptions::from_str(url).map_err(|_| PgOptionsError::InvalidConfiguration)?;
    let host = options.get_host();
    let host = host
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(host);
    let local = options.get_socket().is_some()
        || host.starts_with('/')
        || host == "localhost"
        || host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback());
    if !local {
        options = options.ssl_mode(PgSslMode::VerifyFull);
    }
    Ok(options
        .statement_cache_capacity(16)
        .disable_statement_logging())
}
