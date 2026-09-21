use super::*;
use sqlx::{
    ConnectOptions,
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
};
use std::{str::FromStr, time::Duration};

fn options(value: &str) -> Result<PgConnectOptions> {
    let invalid = || RecurringError::InvalidInput("database URL");
    if value.len() > 8192
        || !(value.starts_with("postgres://") || value.starts_with("postgresql://"))
    {
        return Err(invalid());
    }
    let parsed = url::Url::parse(value).map_err(|_| invalid())?;
    let mut seen = std::collections::BTreeSet::new();
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
            ) || !seen.insert(key.into_owned())
        })
    {
        return Err(invalid());
    }
    let mut options = PgConnectOptions::from_str(value).map_err(|_| invalid())?;
    let host = options.get_host().trim_matches(['[', ']']);
    let local = options.get_socket().is_some()
        || host.starts_with('/')
        || host == "localhost"
        || host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback());
    if !local {
        options = options.ssl_mode(PgSslMode::VerifyFull);
    }
    Ok(options
        .statement_cache_capacity(16)
        .disable_statement_logging())
}

pub(super) async fn open(value: &str) -> Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(5))
        .after_connect(|connection, _| {
            Box::pin(async move {
                for statement in [
                    "SET search_path = public, pg_temp",
                    "SET application_name = 'rullst-messaging-recurring-v1'",
                    "SET synchronous_commit = on",
                    "SET statement_timeout = '5s'",
                    "SET lock_timeout = '5s'",
                    "SET idle_in_transaction_session_timeout = '5s'",
                ] {
                    sqlx::query(statement).execute(&mut *connection).await?;
                }
                Ok(())
            })
        })
        .connect_with(options(value)?)
        .await
        .map_err(|_| RecurringError::Storage)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remote_effective_host_requires_verified_tls_and_unknown_options_are_redacted() {
        for value in [
            "postgres://db.example/mail?sslmode=disable",
            "postgres://localhost/mail?hostaddr=192.0.2.10",
            "postgres://localhost/mail?host=db.example",
        ] {
            assert!(matches!(
                options(value).unwrap().get_ssl_mode(),
                PgSslMode::VerifyFull
            ));
        }
        for value in [
            "sqlite:mail",
            "postgres://db.example/mail?secret_typo=fixture",
            "postgres://db.example/mail?host=a&host=b",
            "postgres://db.example/mail#fragment",
        ] {
            assert!(matches!(
                options(value),
                Err(RecurringError::InvalidInput("database URL"))
            ));
        }
    }
}
