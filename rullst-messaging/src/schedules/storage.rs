use super::*;
use sqlx::Row;

impl<C: Clock> PostgresRecurringStore<C> {
    /// Explicit deployment bootstrap; do not call per request.
    pub async fn initialize(
        url: impl Into<String>,
        config: RecurringConfig,
        keys: MessagingKeyring,
        clock: C,
    ) -> Result<Self> {
        bounded(Self::open(url.into(), config, keys, clock, true)).await
    }
    /// Opens an existing namespace without DDL or missing-state repair.
    pub async fn connect(
        url: impl Into<String>,
        config: RecurringConfig,
        keys: MessagingKeyring,
        clock: C,
    ) -> Result<Self> {
        bounded(Self::open(url.into(), config, keys, clock, false)).await
    }
    async fn open(
        url: String,
        config: RecurringConfig,
        keys: MessagingKeyring,
        clock: C,
        initialize: bool,
    ) -> Result<Self> {
        let store = Self {
            pool: connection::open(&url).await?,
            config,
            keys: Arc::new(keys),
            clock,
        };
        let result = async {
            let started = current(&store.clock)?;
            let mut tx = store
                .pool
                .begin()
                .await
                .map_err(|_| RecurringError::Storage)?;
            if initialize {
                sqlx::query("SELECT pg_catalog.pg_advisory_xact_lock(7311456720483295)")
                    .execute(&mut *tx)
                    .await
                    .map_err(|_| RecurringError::Storage)?;
                for statement in schema::SCHEMA {
                    sqlx::query(*statement)
                        .execute(&mut *tx)
                        .await
                        .map_err(|_| RecurringError::Storage)?;
                }
                let binding = crypto::seal(
                    &store.keys,
                    store.config.namespace(),
                    "configuration",
                    "v1",
                    &store.config.binding(),
                )?;
                sqlx::query(
                    "INSERT INTO rullst_recurring_control(namespace,binding,last_now)
                    VALUES($1,$2,0) ON CONFLICT(namespace) DO NOTHING",
                )
                .bind(store.config.namespace())
                .bind(binding)
                .execute(&mut *tx)
                .await
                .map_err(|_| RecurringError::Storage)?;
            }
            store.commit(tx, started, None).await
        }
        .await;
        if let Err(error) = result {
            store.pool.close().await;
            return Err(error);
        }
        Ok(store)
    }
    /// Closes this pool and all its clones, not independently opened stores.
    pub async fn close(self) {
        self.pool.close().await;
    }
    pub(super) async fn begin(&self) -> Result<(Transaction<'_, Postgres>, i64)> {
        let started = current(&self.clock)?;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| RecurringError::Storage)?;
        let now = self.observe(&mut tx, started).await?;
        Ok((tx, now))
    }
    pub(super) async fn observe(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        minimum: i64,
    ) -> Result<i64> {
        let durable: bool = sqlx::query_scalar(
            "SELECT pg_catalog.current_setting('fsync') = 'on'
            AND pg_catalog.current_setting('full_page_writes') = 'on'
            AND pg_catalog.current_setting('synchronous_commit') IN ('on','remote_apply')
            AND NOT pg_catalog.pg_is_in_recovery()",
        )
        .fetch_one(&mut **tx)
        .await
        .map_err(|_| RecurringError::Storage)?;
        let tables: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pg_catalog.pg_class c
            JOIN pg_catalog.pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public'
            AND c.relkind='r' AND c.relpersistence='p' AND c.relname IN
            ('rullst_recurring_control','rullst_recurring_definitions','rullst_recurring_occurrences')")
            .fetch_one(&mut **tx).await.map_err(|_| RecurringError::Storage)?;
        if !durable || tables != 3 {
            return Err(RecurringError::Configuration);
        }
        let row = sqlx::query(
            "SELECT binding,last_now FROM rullst_recurring_control
            WHERE namespace=$1 FOR UPDATE",
        )
        .bind(self.config.namespace())
        .fetch_optional(&mut **tx)
        .await
        .map_err(|_| RecurringError::Storage)?
        .ok_or(RecurringError::Configuration)?;
        let binding: Vec<u8> = row
            .try_get("binding")
            .map_err(|_| RecurringError::Configuration)?;
        let last: i64 = row
            .try_get("last_now")
            .map_err(|_| RecurringError::Configuration)?;
        if crypto::open(
            &self.keys,
            self.config.namespace(),
            "configuration",
            "v1",
            &binding,
        )?
        .as_slice()
            != self.config.binding()
        {
            return Err(RecurringError::Configuration);
        }
        let now = current(&self.clock)?;
        if now < minimum || now < last || last < 0 {
            return Err(RecurringError::Clock);
        }
        sqlx::query("UPDATE rullst_recurring_control SET last_now=$1 WHERE namespace=$2")
            .bind(now)
            .bind(self.config.namespace())
            .execute(&mut **tx)
            .await
            .map_err(|_| RecurringError::Storage)?;
        Ok(now)
    }
    pub(super) async fn commit(
        &self,
        mut tx: Transaction<'_, Postgres>,
        minimum: i64,
        deadline: Option<i64>,
    ) -> Result<()> {
        let finished = self.observe(&mut tx, minimum).await?;
        if deadline.is_some_and(|end| finished >= end) {
            return Err(RecurringError::InvalidLease);
        }
        tx.commit().await.map_err(|_| RecurringError::Storage)?;
        let observed = current(&self.clock)?;
        if observed < finished {
            return Err(RecurringError::Clock);
        }
        if deadline.is_some_and(|end| observed >= end) {
            return Err(RecurringError::InvalidLease);
        }
        Ok(())
    }
}
