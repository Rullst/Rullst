//! Atomic replay claims across hosts using one authoritative PostgreSQL database.

use super::{
    AgeError, ReplayDurability, ReplayStore,
    replay::{nonce_digest, validate_claim},
};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgSslMode};
use sqlx::{ConnectOptions, PgConnection, PgPool, Postgres, Transaction};
use std::{net::IpAddr, str::FromStr, time::Duration};

const SCHEMA_VERSION: i64 = 1;
const SCHEMA: &[&str] = &[
    "CREATE SCHEMA rullst_age_replay",
    "CREATE TABLE rullst_age_replay.metadata (id INTEGER PRIMARY KEY CHECK(id = 1), schema_version BIGINT NOT NULL, capacity BIGINT NOT NULL CHECK(capacity BETWEEN 1 AND 100000), last_now BIGINT NOT NULL CHECK(last_now >= 0))",
    "CREATE TABLE rullst_age_replay.claims (nonce_digest BYTEA PRIMARY KEY CHECK(octet_length(nonce_digest) = 32), expires_at BIGINT NOT NULL CHECK(expires_at > 0))",
    "CREATE INDEX claims_expiry ON rullst_age_replay.claims(expires_at)",
];

/// Shared durable claims for application hosts using the same writable database.
///
/// Initialization is explicit deployment work; normal connection never recreates
/// missing state. Capacity (1..=100,000) is shared across this fixed schema, not
/// multiplied per pool, host or tenant. Nonces are hashed before storage.
///
/// Operators own database/schema permissions, synchronized host clocks, durable
/// storage and failover fencing. A stale restore or asynchronous replica promotion
/// can resurrect claims: quiesce verifiers and invalidate all outstanding proofs
/// before resuming. Database admission checks cannot prove hardware durability.
#[derive(Clone)]
pub struct PostgresReplayStore {
    pool: PgPool,
    capacity: i64,
}

impl PostgresReplayStore {
    /// Creates a previously absent schema atomically, or validates the existing
    /// one. Never resets an existing schema or changes its capacity. Run with a
    /// deployment role; application roles need only schema usage and table DML.
    pub async fn initialize(url: impl Into<String>, capacity: usize) -> Result<Self, AgeError> {
        Self::open(url.into(), capacity, true).await
    }

    /// Connects to an already initialized schema without DDL or state repair.
    /// Non-loopback TCP connections enforce TLS certificate and hostname checks.
    /// Connection strings and SQL errors are never included in returned errors.
    pub async fn connect(url: impl Into<String>, capacity: usize) -> Result<Self, AgeError> {
        Self::open(url.into(), capacity, false).await
    }

    /// Closes this pool and its clones. Independent application pools remain open.
    pub async fn close(&self) {
        self.pool.close().await;
    }

    async fn open(url: String, capacity: usize, initialize: bool) -> Result<Self, AgeError> {
        if !(1..=100_000).contains(&capacity) {
            return Err(AgeError::InvalidConfiguration);
        }
        let options = connection_options(&url)?;
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .acquire_timeout(Duration::from_secs(5))
            .after_connect(|connection, _| {
                Box::pin(async move {
                    // The private pool keeps caller/server defaults from weakening
                    // claim durability, name resolution or bounded lock waits.
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
            .map_err(|_| AgeError::StoreUnavailable)?;
        let store = Self {
            pool,
            capacity: capacity as i64,
        };
        if let Err(error) = store.prepare(initialize).await {
            store.pool.close().await;
            return Err(error);
        }
        Ok(store)
    }

    async fn begin(&self) -> Result<Transaction<'_, Postgres>, AgeError> {
        self.pool
            .begin_with("BEGIN ISOLATION LEVEL READ COMMITTED READ WRITE")
            .await
            .map_err(|_| AgeError::StoreUnavailable)
    }

    async fn prepare(&self, initialize: bool) -> Result<(), AgeError> {
        let mut tx = self.begin().await?;
        if initialize {
            // Transaction-scoped, database-local bootstrap lock. Claim traffic
            // uses the metadata row instead; no process-local mutex is involved.
            sqlx::query("SELECT pg_catalog.pg_advisory_xact_lock(7311456720483291)")
                .execute(&mut *tx)
                .await
                .map_err(|_| AgeError::StoreUnavailable)?;
            let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_namespace WHERE nspname = 'rullst_age_replay')")
                .fetch_one(&mut *tx).await.map_err(|_| AgeError::StoreUnavailable)?;
            if !exists {
                for statement in SCHEMA {
                    sqlx::query(*statement)
                        .execute(&mut *tx)
                        .await
                        .map_err(|_| AgeError::StoreUnavailable)?;
                }
                sqlx::query("INSERT INTO rullst_age_replay.metadata (id, schema_version, capacity, last_now) VALUES (1, $1, $2, 0)")
                    .bind(SCHEMA_VERSION).bind(self.capacity).execute(&mut *tx).await.map_err(|_| AgeError::StoreUnavailable)?;
            }
        }
        self.storage_configuration(&mut tx).await?;
        self.metadata(&mut tx).await?;
        self.count(&mut tx).await?;
        tx.commit().await.map_err(|_| AgeError::StoreUnavailable)
    }

    async fn storage_configuration(&self, connection: &mut PgConnection) -> Result<(), AgeError> {
        let durable: bool = sqlx::query_scalar("SELECT pg_catalog.current_setting('fsync') = 'on' AND pg_catalog.current_setting('full_page_writes') = 'on' AND pg_catalog.current_setting('synchronous_commit') IN ('on', 'remote_apply') AND NOT pg_catalog.pg_is_in_recovery()")
            .fetch_one(&mut *connection).await.map_err(|_| AgeError::StoreUnavailable)?;
        let tables: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace WHERE n.nspname = 'rullst_age_replay' AND c.relname IN ('metadata', 'claims') AND c.relkind = 'r' AND c.relpersistence = 'p'")
            .fetch_one(connection).await.map_err(|_| AgeError::StoreUnavailable)?;
        if !durable || tables != 2 {
            return Err(AgeError::StoreConfiguration);
        }
        Ok(())
    }

    async fn metadata(&self, connection: &mut PgConnection) -> Result<i64, AgeError> {
        let row: Option<(i64, i64, i64)> = sqlx::query_as("SELECT schema_version, capacity, last_now FROM rullst_age_replay.metadata WHERE id = 1 FOR UPDATE")
            .fetch_optional(connection).await.map_err(|_| AgeError::StoreUnavailable)?;
        match row {
            Some((SCHEMA_VERSION, capacity, last_now))
                if capacity == self.capacity && last_now >= 0 =>
            {
                Ok(last_now)
            }
            _ => Err(AgeError::StoreConfiguration),
        }
    }

    async fn count(&self, connection: &mut PgConnection) -> Result<i64, AgeError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rullst_age_replay.claims")
            .fetch_one(connection)
            .await
            .map_err(|_| AgeError::StoreUnavailable)?;
        if !(0..=self.capacity).contains(&count) {
            return Err(AgeError::StoreConfiguration);
        }
        Ok(count)
    }

    async fn claim_in_transaction(
        &self,
        connection: &mut PgConnection,
        nonce: [u8; 32],
        expires_at: i64,
        now: i64,
    ) -> Result<bool, AgeError> {
        self.storage_configuration(connection).await?;
        if now < self.metadata(connection).await? {
            return Err(AgeError::ClockRollback);
        }
        sqlx::query("UPDATE rullst_age_replay.metadata SET last_now = $1 WHERE id = 1")
            .bind(now)
            .execute(&mut *connection)
            .await
            .map_err(|_| AgeError::StoreUnavailable)?;
        sqlx::query("DELETE FROM rullst_age_replay.claims WHERE expires_at <= $1")
            .bind(now)
            .execute(&mut *connection)
            .await
            .map_err(|_| AgeError::StoreUnavailable)?;
        let digest = nonce_digest(&nonce);
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM rullst_age_replay.claims WHERE nonce_digest = $1)",
        )
        .bind(digest.as_ref())
        .fetch_one(&mut *connection)
        .await
        .map_err(|_| AgeError::StoreUnavailable)?;
        if exists {
            return Ok(false);
        }
        if self.count(connection).await? >= self.capacity {
            return Err(AgeError::StoreCapacity);
        }
        sqlx::query(
            "INSERT INTO rullst_age_replay.claims (nonce_digest, expires_at) VALUES ($1, $2)",
        )
        .bind(digest.as_ref())
        .bind(expires_at)
        .execute(connection)
        .await
        .map_err(|_| AgeError::StoreUnavailable)?;
        Ok(true)
    }
}

impl ReplayStore for PostgresReplayStore {
    fn durability(&self) -> ReplayDurability {
        ReplayDurability::SharedDurable
    }

    async fn claim(&self, nonce: [u8; 32], expires_at: i64, now: i64) -> Result<bool, AgeError> {
        validate_claim(expires_at, now)?;
        let mut tx = self.begin().await?;
        match self
            .claim_in_transaction(&mut tx, nonce, expires_at, now)
            .await
        {
            Ok(claimed) => {
                tx.commit().await.map_err(|_| AgeError::StoreUnavailable)?;
                Ok(claimed)
            }
            Err(error) => {
                tx.rollback()
                    .await
                    .map_err(|_| AgeError::StoreUnavailable)?;
                Err(error)
            }
        }
    }
}

fn connection_options(url: &str) -> Result<PgConnectOptions, AgeError> {
    if url.len() > 8192 || !(url.starts_with("postgres://") || url.starts_with("postgresql://")) {
        return Err(AgeError::InvalidConfiguration);
    }
    // SQLx logs unknown query parameter values. Reject them before invoking its
    // parser so a misspelled password/token option cannot enter ordinary logs.
    let parsed = url::Url::parse(url).map_err(|_| AgeError::InvalidConfiguration)?;
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
        return Err(AgeError::InvalidConfiguration);
    }
    let mut options =
        PgConnectOptions::from_str(url).map_err(|_| AgeError::InvalidConfiguration)?;
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
        .application_name("rullst-age-replay-v1")
        .statement_cache_capacity(16)
        .disable_statement_logging())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_connections_cannot_disable_certificate_or_hostname_verification() {
        for mode in ["disable", "prefer", "require", "verify-ca", "verify-full"] {
            let options = connection_options(&format!(
                "postgres://fixture:secret@db.example.test/replay?sslmode={mode}"
            ))
            .unwrap();
            assert!(matches!(options.get_ssl_mode(), PgSslMode::VerifyFull));
        }
        for host in ["127.0.0.1", "[::1]", "localhost"] {
            assert!(matches!(
                connection_options(&format!(
                    "postgres://postgres@{host}/replay?sslmode=disable"
                ))
                .unwrap()
                .get_ssl_mode(),
                PgSslMode::Disable
            ));
        }
        for url in [
            "",
            "mock_local",
            "sqlite:replay",
            "https://db.example.test/replay",
            "postgres://[malformed-secret",
            "postgres://localhost/replay?passwrod=must-not-be-logged",
            "postgres://localhost/replay?password%5Ftypo=must-not-be-logged",
            "postgres://localhost/replay?sslmode=disable&sslmode=prefer",
            "postgres://localhost/replay#ignored-credential",
        ] {
            assert!(matches!(
                connection_options(url),
                Err(AgeError::InvalidConfiguration)
            ));
        }
    }
}
