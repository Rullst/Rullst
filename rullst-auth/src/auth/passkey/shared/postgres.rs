//! One authoritative PostgreSQL schema for bounded, single-use ceremonies.
use super::{
    CeremonyClock, CeremonyStoreConfig, PasskeyCeremonyError as Error, SystemCeremonyClock,
};
use sqlx::{
    ConnectOptions, PgPool, Postgres, Transaction,
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
};
use std::{net::IpAddr, str::FromStr, time::Duration};
mod operations;

const SCHEMA: &[&str] = &[
    "CREATE SCHEMA rullst_passkey_ceremony",
    "CREATE TABLE rullst_passkey_ceremony.metadata (id INTEGER PRIMARY KEY CHECK(id=1), version BIGINT NOT NULL CHECK(version=1), epoch TEXT NOT NULL CHECK(octet_length(epoch) BETWEEN 1 AND 128), capacity BIGINT NOT NULL CHECK(capacity BETWEEN 1 AND 100000), lifetime BIGINT NOT NULL CHECK(lifetime BETWEEN 1 AND 600), last_now BIGINT NOT NULL CHECK(last_now>=0))",
    "CREATE TABLE rullst_passkey_ceremony.pending (challenge BYTEA PRIMARY KEY CHECK(octet_length(challenge)=32), binding BYTEA NOT NULL CHECK(octet_length(binding)=32), kind SMALLINT NOT NULL CHECK(kind IN (1,2)), credentials BYTEA NOT NULL CHECK(octet_length(credentials)<=1024), issued_at BIGINT NOT NULL CHECK(issued_at>=0), expires_at BIGINT NOT NULL CHECK(expires_at>issued_at))",
    "CREATE INDEX pending_expiry ON rullst_passkey_ceremony.pending(expires_at)",
];

/// Separate hosts share one writable database. Operators own clock synchronization,
/// primary/failover fencing, schema permissions, TLS trust and backup/restore policy.
#[derive(Clone)]
pub struct PostgresCeremonyStore<C = SystemCeremonyClock> {
    pool: PgPool,
    config: CeremonyStoreConfig,
    clock: C,
}
impl PostgresCeremonyStore {
    /// Explicit deployment initialization; existing state is validated, never repaired.
    pub async fn initialize(
        url: impl Into<String>,
        config: CeremonyStoreConfig,
    ) -> Result<Self, Error> {
        Self::initialize_with_clock(url, config, SystemCeremonyClock).await
    }
    /// Opens only existing state; no fallback, schema creation or metadata reset.
    pub async fn connect(
        url: impl Into<String>,
        config: CeremonyStoreConfig,
    ) -> Result<Self, Error> {
        Self::connect_with_clock(url, config, SystemCeremonyClock).await
    }
}
impl<C: CeremonyClock> PostgresCeremonyStore<C> {
    pub async fn initialize_with_clock(
        url: impl Into<String>,
        config: CeremonyStoreConfig,
        clock: C,
    ) -> Result<Self, Error> {
        tokio::time::timeout(
            Duration::from_secs(12),
            Self::open(url.into(), config, clock, true),
        )
        .await
        .map_err(|_| Error::UncertainCommit)?
    }
    pub async fn connect_with_clock(
        url: impl Into<String>,
        config: CeremonyStoreConfig,
        clock: C,
    ) -> Result<Self, Error> {
        tokio::time::timeout(
            Duration::from_secs(12),
            Self::open(url.into(), config, clock, false),
        )
        .await
        .map_err(|_| Error::UncertainCommit)?
    }
    /// Closes this pool and its clones; independent pools remain open.
    pub async fn close(&self) {
        self.pool.close().await;
    }

    async fn open(
        url: String,
        config: CeremonyStoreConfig,
        clock: C,
        initialize: bool,
    ) -> Result<Self, Error> {
        let options = connection_options(&url)?;
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .acquire_timeout(Duration::from_secs(5))
            .after_connect(|connection, _| {
                Box::pin(async move {
                    for statement in [
                        "SET search_path = pg_catalog",
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
            .connect_with(options)
            .await
            .map_err(|_| Error::Unavailable)?;
        let store = Self {
            pool,
            config,
            clock,
        };
        if let Err(error) = store.prepare(initialize).await {
            store.close().await;
            return Err(error);
        }
        Ok(store)
    }
    async fn transaction(&self) -> Result<Transaction<'_, Postgres>, Error> {
        self.pool
            .begin_with("BEGIN ISOLATION LEVEL READ COMMITTED READ WRITE")
            .await
            .map_err(|_| Error::Unavailable)
    }
    async fn prepare(&self, initialize: bool) -> Result<(), Error> {
        if initialize {
            let mut tx = self.transaction().await?;
            sqlx::query("SELECT pg_catalog.pg_advisory_xact_lock(7311456720483292)")
                .execute(&mut *tx)
                .await
                .map_err(|_| Error::Unavailable)?;
            let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_namespace WHERE nspname='rullst_passkey_ceremony')")
                .fetch_one(&mut *tx).await.map_err(|_| Error::Unavailable)?;
            if !exists {
                for &statement in SCHEMA {
                    sqlx::query(statement)
                        .execute(&mut *tx)
                        .await
                        .map_err(|_| Error::Unavailable)?;
                }
                sqlx::query("INSERT INTO rullst_passkey_ceremony.metadata (id,version,epoch,capacity,lifetime,last_now) VALUES (1,1,$1,$2,$3,0)")
                    .bind(self.config.epoch()).bind(i64::from(self.config.capacity())).bind(i64::from(self.config.lifetime_seconds()))
                    .execute(&mut *tx).await.map_err(|_| Error::Unavailable)?;
            }
            tx.commit().await.map_err(|_| Error::UncertainCommit)?;
        }
        let mut op = self.begin().await?;
        op.count().await?;
        op.finish().await
    }
}
fn connection_options(url: &str) -> Result<PgConnectOptions, Error> {
    if url.len() > 8192 || !(url.starts_with("postgres://") || url.starts_with("postgresql://")) {
        return Err(Error::Configuration);
    }
    // SQLx logs unknown query parameter values. Reject them before invoking its
    // parser so a misspelled password/token option cannot enter ordinary logs.
    let parsed = url::Url::parse(url).map_err(|_| Error::Configuration)?;
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
        return Err(Error::Configuration);
    }
    let mut options = PgConnectOptions::from_str(url).map_err(|_| Error::Configuration)?;
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
        .application_name("rullst-passkey-ceremony-v1")
        .statement_cache_capacity(16)
        .disable_statement_logging())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remote_tls_cannot_be_downgraded_and_unknown_options_are_rejected_before_sqlx_logs() {
        for mode in ["disable", "prefer", "require"] {
            let options = connection_options(&format!(
                "postgres://fixture:secret@db.example.test/ceremonies?sslmode={mode}"
            ))
            .unwrap();
            assert!(matches!(options.get_ssl_mode(), PgSslMode::VerifyFull));
        }
        for host in ["127.0.0.1", "[::1]", "localhost"] {
            assert!(matches!(
                connection_options(&format!(
                    "postgres://postgres@{host}/ceremonies?sslmode=disable"
                ))
                .unwrap()
                .get_ssl_mode(),
                PgSslMode::Disable
            ));
        }
        for value in [
            "mock_local",
            "postgres://localhost/db?passwrod=secret",
            "postgres://localhost/db?password%5Ftypo=secret",
            "postgres://localhost/db?sslmode=disable&sslmode=prefer",
            "postgres://localhost/db#secret",
        ] {
            assert!(matches!(
                connection_options(value),
                Err(Error::Configuration)
            ));
        }
    }
}
