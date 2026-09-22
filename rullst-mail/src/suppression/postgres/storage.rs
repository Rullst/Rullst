use super::*;
use sqlx::Row;

const SCHEMA: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS rullst_mail_pg_suppression_control (
        namespace TEXT PRIMARY KEY, binding BYTEA NOT NULL CHECK(octet_length(binding) = 32),
        max_recipients BIGINT NOT NULL CHECK(max_recipients BETWEEN 1 AND 1000000),
        max_events BIGINT NOT NULL CHECK(max_events BETWEEN 1 AND 1000000),
        last_now BIGINT NOT NULL CHECK(last_now >= 0))",
    "CREATE TABLE IF NOT EXISTS rullst_mail_pg_suppression_recipients (
        namespace TEXT NOT NULL, recipient_tag BYTEA NOT NULL CHECK(octet_length(recipient_tag) = 32),
        reason BIGINT NOT NULL CHECK(reason BETWEEN 1 AND 3), provider TEXT NOT NULL,
        first_seen_at BIGINT NOT NULL CHECK(first_seen_at > 0),
        last_seen_at BIGINT NOT NULL CHECK(last_seen_at >= first_seen_at),
        PRIMARY KEY(namespace,recipient_tag))",
    "CREATE TABLE IF NOT EXISTS rullst_mail_pg_suppression_events (
        namespace TEXT NOT NULL, event_tag BYTEA NOT NULL CHECK(octet_length(event_tag) = 32),
        fingerprint BYTEA NOT NULL CHECK(octet_length(fingerprint) = 32),
        observed_at BIGINT NOT NULL CHECK(observed_at > 0),
        PRIMARY KEY(namespace,event_tag))",
    "CREATE INDEX IF NOT EXISTS rullst_mail_pg_suppression_event_time
        ON rullst_mail_pg_suppression_events(namespace,observed_at)",
];

impl PostgresSuppressionStore {
    /// Explicit deployment bootstrap. Creates fixed tables and binds this
    /// namespace to its key/quotas. Never perform initialization per request.
    pub async fn initialize(
        url: impl Into<String>,
        key: SuppressionKey,
        config: PostgresSuppressionConfig,
    ) -> Result<Self, SuppressionError> {
        bounded(Self::open(url.into(), key, config, true)).await
    }
    /// Opens existing namespace state without DDL, quota drift or repair.
    pub async fn connect(
        url: impl Into<String>,
        key: SuppressionKey,
        config: PostgresSuppressionConfig,
    ) -> Result<Self, SuppressionError> {
        bounded(Self::open(url.into(), key, config, false)).await
    }
    async fn open(
        url: String,
        key: SuppressionKey,
        config: PostgresSuppressionConfig,
        initialize: bool,
    ) -> Result<Self, SuppressionError> {
        let pool = connection::open(&url).await?;
        let store = Self { pool, config, key };
        let result = async {
            let current = now()?;
            let mut tx = store
                .pool
                .begin()
                .await
                .map_err(|_| unavailable("begin startup"))?;
            if initialize {
                sqlx::query("SELECT pg_catalog.pg_advisory_xact_lock(7311456720483294)")
                    .execute(&mut *tx)
                    .await
                    .map_err(|_| unavailable("bootstrap lock"))?;
                for statement in SCHEMA {
                    sqlx::query(*statement)
                        .execute(&mut *tx)
                        .await
                        .map_err(|_| unavailable("bootstrap schema"))?;
                }
                sqlx::query(
                    "INSERT INTO rullst_mail_pg_suppression_control
                     (namespace,binding,max_recipients,max_events,last_now)
                     VALUES ($1,$2,$3,$4,0) ON CONFLICT(namespace) DO NOTHING",
                )
                .bind(&store.config.namespace)
                .bind(store.binding()?)
                .bind(store.config.max_recipients as i64)
                .bind(store.config.max_events as i64)
                .execute(&mut *tx)
                .await
                .map_err(|_| unavailable("initialize namespace"))?;
            }
            store.commit(tx, current).await
        }
        .await;
        if let Err(error) = result {
            store.pool.close().await;
            return Err(error);
        }
        Ok(store)
    }
    /// Closes this pool and its clones; separately opened stores remain usable.
    pub async fn close(self) {
        self.pool.close().await;
    }

    pub(super) async fn begin(&self) -> Result<(Transaction<'_, Postgres>, i64), SuppressionError> {
        let started = now()?;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| unavailable("begin operation"))?;
        let current = self.observe(&mut tx, started).await?;
        Ok((tx, current))
    }
    async fn observe(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        minimum: i64,
    ) -> Result<i64, SuppressionError> {
        self.durable(tx).await?;
        let row = sqlx::query(
            "SELECT binding,max_recipients,max_events,last_now
             FROM rullst_mail_pg_suppression_control WHERE namespace = $1 FOR UPDATE",
        )
        .bind(&self.config.namespace)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|_| unavailable("namespace lock"))?
        .ok_or(SuppressionError::InvalidConfiguration("missing namespace"))?;
        let binding: Vec<u8> = row
            .try_get("binding")
            .map_err(|_| unavailable("namespace binding"))?;
        let recipients: i64 = row
            .try_get("max_recipients")
            .map_err(|_| unavailable("namespace quota"))?;
        let events: i64 = row
            .try_get("max_events")
            .map_err(|_| unavailable("namespace quota"))?;
        let last: i64 = row
            .try_get("last_now")
            .map_err(|_| unavailable("namespace clock"))?;
        if !self.matches(
            b"configuration",
            &[
                &(self.config.max_recipients as u64).to_be_bytes(),
                &(self.config.max_events as u64).to_be_bytes(),
            ],
            &binding,
        )? || recipients != self.config.max_recipients as i64
            || events != self.config.max_events as i64
            || last < 0
        {
            return Err(SuppressionError::InvalidConfiguration("namespace binding"));
        }
        let current = now()?;
        if current < last || current < minimum {
            return Err(SuppressionError::InvalidConfiguration("server clock"));
        }
        sqlx::query(
            "UPDATE rullst_mail_pg_suppression_control SET last_now = $1 WHERE namespace = $2",
        )
        .bind(current)
        .bind(&self.config.namespace)
        .execute(&mut **tx)
        .await
        .map_err(|_| unavailable("record server clock"))?;
        Ok(current)
    }
    pub(super) async fn commit(
        &self,
        mut tx: Transaction<'_, Postgres>,
        started: i64,
    ) -> Result<(), SuppressionError> {
        let finished = self.observe(&mut tx, started).await?;
        tx.commit()
            .await
            .map_err(|_| unavailable("commit operation"))?;
        if now()? < finished {
            return Err(SuppressionError::InvalidConfiguration("server clock"));
        }
        Ok(())
    }
    async fn durable(&self, tx: &mut Transaction<'_, Postgres>) -> Result<(), SuppressionError> {
        let durable: bool = sqlx::query_scalar(
            "SELECT pg_catalog.current_setting('fsync') = 'on'
             AND pg_catalog.current_setting('full_page_writes') = 'on'
             AND pg_catalog.current_setting('synchronous_commit') IN ('on','remote_apply')
             AND NOT pg_catalog.pg_is_in_recovery()",
        )
        .fetch_one(&mut **tx)
        .await
        .map_err(|_| unavailable("database durability"))?;
        let tables: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace n
             ON n.oid = c.relnamespace WHERE n.nspname = 'public' AND c.relkind = 'r'
             AND c.relpersistence = 'p' AND c.relname IN
             ('rullst_mail_pg_suppression_control','rullst_mail_pg_suppression_recipients',
              'rullst_mail_pg_suppression_events')",
        )
        .fetch_one(&mut **tx)
        .await
        .map_err(|_| unavailable("database tables"))?;
        if !durable || tables != 3 {
            return Err(SuppressionError::InvalidConfiguration("durable database"));
        }
        Ok(())
    }
    pub(super) async fn counts(
        &self,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<(usize, usize), SuppressionError> {
        let (recipients, events): (i64, i64) = sqlx::query_as(
            "SELECT (SELECT COUNT(*) FROM rullst_mail_pg_suppression_recipients
                     WHERE namespace = $1),
                    (SELECT COUNT(*) FROM rullst_mail_pg_suppression_events
                     WHERE namespace = $1)",
        )
        .bind(&self.config.namespace)
        .fetch_one(&mut **tx)
        .await
        .map_err(|_| unavailable("count state"))?;
        let recipients = usize::try_from(recipients)
            .map_err(|_| SuppressionError::CorruptStorage("recipient count"))?;
        let events =
            usize::try_from(events).map_err(|_| SuppressionError::CorruptStorage("event count"))?;
        if recipients > self.config.max_recipients || events > self.config.max_events {
            return Err(SuppressionError::CorruptStorage("state quota"));
        }
        Ok((recipients, events))
    }
}
