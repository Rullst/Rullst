use super::*;
use sqlx::any::AnyConnectOptions;
use std::str::FromStr;
use url::Url;

const MAX_URL_BYTES: usize = 8192;

/// Parses a PostgreSQL URL with the policy shared by every recovery-family
/// pool: bounded length, no fragment, allowlisted connection/TLS options and
/// `sslmode=verify-full` for every non-loopback, non-socket host.
pub(super) fn postgres_options(url: &str) -> Result<AnyConnectOptions, RecoveryError> {
    if url.len() > MAX_URL_BYTES {
        return Err(RecoveryError::Configuration);
    }
    let mut url = Url::parse(url).map_err(|_| RecoveryError::Configuration)?;
    if url.fragment().is_some() || !matches!(url.scheme(), "postgres" | "postgresql") {
        return Err(RecoveryError::Configuration);
    }
    configure_postgres(&mut url)?;
    AnyConnectOptions::from_str(url.as_str()).map_err(|_| RecoveryError::Configuration)
}

#[cfg(any(
    feature = "email-login-sqlite",
    feature = "email-login-postgres",
    feature = "api-tokens-sqlite",
    feature = "api-tokens-postgres"
))]
pub(super) async fn open(
    url: String,
    keys: RecoverySecrets,
    initialize: bool,
) -> Result<(SqlRecoveryStore, bool), RecoveryError> {
    use sqlx::ConnectOptions;
    use std::{sync::Arc, time::Duration};

    if url.len() > MAX_URL_BYTES {
        return Err(RecoveryError::Configuration);
    }
    let mut url = Url::parse(&url).map_err(|_| RecoveryError::Configuration)?;
    if url.fragment().is_some() {
        return Err(RecoveryError::Configuration);
    }
    let postgres = matches!(url.scheme(), "postgres" | "postgresql");
    if postgres {
        configure_postgres(&mut url)?;
    } else {
        configure_sqlite(&mut url, initialize)?;
    }
    sqlx::any::install_default_drivers();
    let options = AnyConnectOptions::from_str(url.as_str())
        .map_err(|_| RecoveryError::Configuration)?
        .disable_statement_logging();
    let pool = sqlx::any::AnyPoolOptions::new()
        .max_connections(if postgres { 4 } else { 1 })
        .acquire_timeout(Duration::from_secs(5))
        .after_connect(move |connection, _| {
            Box::pin(async move {
                let statements: &[&'static str] = if postgres {
                    &[
                        "SET search_path = public, pg_temp",
                        "SET application_name = 'rullst-auth-state-v1'",
                        "SET synchronous_commit = on",
                        "SET statement_timeout = '5s'",
                        "SET lock_timeout = '5s'",
                        "SET idle_in_transaction_session_timeout = '5s'",
                    ]
                } else {
                    &[
                        "PRAGMA busy_timeout = 5000",
                        "PRAGMA journal_mode = WAL",
                        "PRAGMA synchronous = FULL",
                        "PRAGMA foreign_keys = ON",
                    ]
                };
                for statement in statements {
                    sqlx::query(*statement).execute(&mut *connection).await?;
                }
                Ok(())
            })
        })
        .connect_with(options)
        .await?;
    Ok((
        SqlRecoveryStore {
            pool,
            keys: Arc::new(keys),
            instance: Arc::new(()),
            reset_failures: super::failures::FailureBudget::password_reset(),
        },
        postgres,
    ))
}

#[cfg(feature = "recovery-postgres")]
fn configure_postgres(url: &mut Url) -> Result<(), RecoveryError> {
    let mut seen = std::collections::BTreeSet::new();
    if url.query_pairs().any(|(key, _)| {
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
        ) || !seen.insert(key.into_owned())
    }) {
        return Err(RecoveryError::Configuration);
    }
    // Parse only known options so SQLx cannot log an unknown credential field.
    let parsed = sqlx::postgres::PgConnectOptions::from_str(url.as_str())
        .map_err(|_| RecoveryError::Configuration)?;
    let host = parsed.get_host().trim_matches(['[', ']']);
    let local = parsed.get_socket().is_some()
        || host.starts_with('/')
        || host == "localhost"
        || host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback());
    if !local {
        let pairs: Vec<(String, String)> = url
            .query_pairs()
            .filter(|(key, _)| key != "sslmode" && key != "ssl-mode")
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect();
        url.set_query(None);
        url.query_pairs_mut()
            .extend_pairs(pairs)
            .append_pair("sslmode", "verify-full");
    }
    Ok(())
}

#[cfg(not(feature = "recovery-postgres"))]
fn configure_postgres(_: &mut Url) -> Result<(), RecoveryError> {
    Err(RecoveryError::Configuration)
}

#[cfg(any(feature = "email-login-sqlite", feature = "api-tokens-sqlite"))]
fn configure_sqlite(url: &mut Url, initialize: bool) -> Result<(), RecoveryError> {
    if url.scheme() != "sqlite" || url.host_str().is_some() {
        return Err(RecoveryError::Configuration);
    }
    let mut seen = std::collections::BTreeSet::new();
    if url
        .query_pairs()
        .any(|(key, _)| !matches!(key.as_ref(), "mode" | "cache") || !seen.insert(key.into_owned()))
    {
        return Err(RecoveryError::Configuration);
    }
    let options = sqlx::sqlite::SqliteConnectOptions::from_str(url.as_str())
        .map_err(|_| RecoveryError::Configuration)?;
    if url.path() == ":memory:"
        || options.get_filename().as_os_str().is_empty()
        || options.get_filename() == std::path::Path::new(":memory:")
        || options
            .get_filename()
            .to_string_lossy()
            .starts_with("file:")
        || url.query_pairs().any(|(key, value)| {
            (key == "mode" && value == "memory") || (key == "cache" && value == "shared")
        })
    {
        return Err(RecoveryError::Configuration);
    }
    url.set_query(None);
    url.query_pairs_mut()
        .append_pair("mode", if initialize { "rwc" } else { "rw" })
        .append_pair("cache", "private");
    Ok(())
}

#[cfg(all(
    any(feature = "email-login-postgres", feature = "api-tokens-postgres"),
    not(any(feature = "email-login-sqlite", feature = "api-tokens-sqlite"))
))]
fn configure_sqlite(_: &mut Url, _: bool) -> Result<(), RecoveryError> {
    Err(RecoveryError::Configuration)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[cfg(feature = "recovery-postgres")]
    #[test]
    fn remote_transport_cannot_disable_certificate_or_hostname_verification() {
        for address in [
            "postgres://db.example/accounts?sslmode=disable",
            "postgres://db.example/accounts?ssl-mode=require",
            "postgres://localhost/accounts?host=db.example&sslmode=disable",
        ] {
            let mut url = Url::parse(address).unwrap();
            configure_postgres(&mut url).unwrap();
            let parsed = sqlx::postgres::PgConnectOptions::from_str(url.as_str()).unwrap();
            assert!(matches!(
                parsed.get_ssl_mode(),
                sqlx::postgres::PgSslMode::VerifyFull
            ));
        }
        for address in [
            "postgres://db.example/accounts?unrecognized=redacted",
            "postgres://db.example/accounts?host=a&host=b",
        ] {
            assert_eq!(
                configure_postgres(&mut Url::parse(address).unwrap()),
                Err(RecoveryError::Configuration)
            );
        }
    }

    #[cfg(feature = "recovery-postgres")]
    fn ssl_mode(options: &AnyConnectOptions) -> sqlx::postgres::PgSslMode {
        sqlx::postgres::PgConnectOptions::from_str(options.database_url.as_str())
            .unwrap()
            .get_ssl_mode()
    }

    #[cfg(feature = "recovery-postgres")]
    #[test]
    fn recovery_store_postgres_options_verify_remote_tls_and_keep_loopback() {
        use sqlx::postgres::PgSslMode;
        // No sslmode used to mean SQLx `prefer`: unverified and plaintext-capable.
        for address in [
            "postgres://user:secret@db.example/accounts",
            "postgresql://db.example:6543/accounts?sslmode=prefer",
            "postgres://db.example/accounts?sslmode=disable&sslrootcert=%2Fetc%2Fca.pem",
            "postgres://[2001:db8::1]/accounts?sslmode=require",
        ] {
            let options = postgres_options(address).unwrap();
            assert!(
                matches!(ssl_mode(&options), PgSslMode::VerifyFull),
                "{address}"
            );
        }
        let custom_ca =
            postgres_options("postgres://db.example/a?sslrootcert=%2Fetc%2Fca.pem").unwrap();
        assert!(
            custom_ca
                .database_url
                .query_pairs()
                .any(|(key, value)| key == "sslrootcert" && value == "/etc/ca.pem")
        );
        // Loopback and Unix-socket development connections keep the caller's mode.
        for (address, expected) in [
            (
                "postgres://localhost/accounts?sslmode=disable",
                PgSslMode::Disable,
            ),
            ("postgres://127.0.0.1/accounts", PgSslMode::Prefer),
            (
                "postgres://[::1]/accounts?sslmode=require",
                PgSslMode::Require,
            ),
            (
                "postgres:///accounts?host=%2Fvar%2Frun%2Fpostgresql",
                PgSslMode::Prefer,
            ),
        ] {
            let options = postgres_options(address).unwrap();
            assert_eq!(
                std::mem::discriminant(&ssl_mode(&options)),
                std::mem::discriminant(&expected),
                "{address}"
            );
        }
        for address in [
            "postgres://db.example/accounts?application_name=recovery",
            "postgres://db.example/accounts?sslmode=disable&sslmode=disable",
            "postgres://db.example/accounts#fragment",
            "sqlite:accounts.db",
            "not a url",
        ] {
            assert_eq!(
                postgres_options(address).err(),
                Some(RecoveryError::Configuration),
                "{address}"
            );
        }
        let oversized = format!("postgres://db.example/{}", "a".repeat(MAX_URL_BYTES));
        assert_eq!(
            postgres_options(&oversized).err(),
            Some(RecoveryError::Configuration)
        );
    }

    #[cfg(any(feature = "email-login-sqlite", feature = "api-tokens-sqlite"))]
    #[test]
    fn volatile_ambiguous_and_unknown_sqlite_options_fail_before_connecting() {
        for address in [
            "sqlite::memory:",
            "sqlite:file::memory:",
            "sqlite:file%3Atest%3Fmode%3Dmemory",
            "sqlite:database?mode=memory",
            "sqlite:database?cache=shared",
            "sqlite://host/database",
            "sqlite:database?unknown=value",
            "sqlite:database?mode=rw&mode=rwc",
            "https://app.example/database",
        ] {
            assert_eq!(
                configure_sqlite(&mut Url::parse(address).unwrap(), true),
                Err(RecoveryError::Configuration)
            );
        }
        let mut url = Url::parse("sqlite:database?mode=rwc").unwrap();
        configure_sqlite(&mut url, false).unwrap();
        assert!(
            url.query_pairs()
                .any(|(key, value)| key == "mode" && value == "rw")
        );
        assert!(
            url.query_pairs()
                .any(|(key, value)| key == "cache" && value == "private")
        );
    }
}
