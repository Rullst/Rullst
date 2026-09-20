use super::{Error, PostgresCeremonyStore};
use crate::auth::passkey::shared::{
    CeremonyClock, CeremonyDurability, CeremonyIntent, CeremonyKind, CeremonyStoreConfig,
    ConsumedCeremony, PasskeyCeremonyStore, contracts::MAX_TIME,
};
use sqlx::{Postgres, Transaction};
use subtle::ConstantTimeEq;

pub(super) struct Operation<'a, C> {
    store: &'a PostgresCeremonyStore<C>,
    tx: Transaction<'a, Postgres>,
    now: i64,
    deadline: Option<i64>,
}
impl<C: CeremonyClock> PostgresCeremonyStore<C> {
    pub(super) async fn begin(&self) -> Result<Operation<'_, C>, Error> {
        let mut tx = self.transaction().await?;
        let durable: bool = sqlx::query_scalar("SELECT pg_catalog.current_setting('fsync')='on' AND pg_catalog.current_setting('full_page_writes')='on' AND pg_catalog.current_setting('synchronous_commit') IN ('on','remote_apply') AND NOT pg_catalog.pg_is_in_recovery()")
            .fetch_one(&mut *tx).await.map_err(|_| Error::Unavailable)?;
        let tables: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='rullst_passkey_ceremony' AND c.relname IN ('metadata','pending') AND c.relkind='r' AND c.relpersistence='p'")
            .fetch_one(&mut *tx).await.map_err(|_| Error::Unavailable)?;
        if !durable || tables != 2 {
            return Err(Error::Configuration);
        }
        let row: Option<(i64,String,i64,i64,i64)> = sqlx::query_as("SELECT version,pg_catalog.substr(epoch,1,129),capacity,lifetime,last_now FROM rullst_passkey_ceremony.metadata WHERE id=1 FOR UPDATE")
            .fetch_optional(&mut *tx).await.map_err(|_| Error::Unavailable)?;
        let Some((1, epoch, capacity, lifetime, last_now)) = row else {
            return Err(Error::Corrupt);
        };
        if epoch != self.config.epoch()
            || capacity != i64::from(self.config.capacity())
            || lifetime != i64::from(self.config.lifetime_seconds())
        {
            return Err(Error::Configuration);
        }
        let now = self.clock.now()?;
        if !(0..=MAX_TIME).contains(&now) || last_now < 0 || now < last_now {
            return Err(Error::Corrupt);
        }
        Ok(Operation {
            store: self,
            tx,
            now,
            deadline: None,
        })
    }
}
impl<C: CeremonyClock> Operation<'_, C> {
    pub(super) async fn count(&mut self) -> Result<i64, Error> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rullst_passkey_ceremony.pending")
            .fetch_one(&mut *self.tx)
            .await
            .map_err(|_| Error::Unavailable)?;
        if count > i64::from(self.store.config.capacity()) {
            return Err(Error::Corrupt);
        }
        Ok(count)
    }
    fn until(&mut self, expires_at: i64) -> Result<(), Error> {
        if expires_at <= self.now {
            return Err(Error::Expired);
        }
        self.deadline = Some(self.deadline.map_or(expires_at, |old| old.min(expires_at)));
        Ok(())
    }
    fn current(&self) -> Result<i64, Error> {
        let now = self.store.clock.now()?;
        if now < self.now || now > MAX_TIME {
            return Err(Error::Corrupt);
        }
        if self.deadline.is_some_and(|deadline| now >= deadline) {
            return Err(Error::Expired);
        }
        Ok(now)
    }
    pub(super) async fn finish(mut self) -> Result<(), Error> {
        let now = self.current()?;
        sqlx::query("UPDATE rullst_passkey_ceremony.metadata SET last_now=$1 WHERE id=1")
            .bind(now)
            .execute(&mut *self.tx)
            .await
            .map_err(|_| Error::Unavailable)?;
        self.current()?;
        self.tx.commit().await.map_err(|_| Error::UncertainCommit)?;
        let after = self.store.clock.now().map_err(|_| Error::UncertainCommit)?;
        if after < now
            || after > MAX_TIME
            || self.deadline.is_some_and(|deadline| after >= deadline)
        {
            return Err(Error::UncertainCommit);
        }
        Ok(())
    }
}
impl<C: CeremonyClock> PasskeyCeremonyStore for PostgresCeremonyStore<C> {
    fn config(&self) -> &CeremonyStoreConfig {
        &self.config
    }
    fn durability(&self) -> CeremonyDurability {
        CeremonyDurability::SharedDurable
    }
    async fn issue(&self, intent: &CeremonyIntent) -> Result<(), Error> {
        bounded(async {
            let mut op = self.begin().await?;
            op.count().await?;
            sqlx::query("DELETE FROM rullst_passkey_ceremony.pending WHERE expires_at<=$1")
                .bind(op.now)
                .execute(&mut *op.tx)
                .await
                .map_err(|_| Error::Unavailable)?;
            if op.count().await? >= i64::from(self.config.capacity()) {
                return Err(Error::Capacity);
            }
            let expires = op
                .now
                .checked_add(i64::from(self.config.lifetime_seconds()))
                .filter(|v| *v <= MAX_TIME)
                .ok_or(Error::InvalidInput)?;
            let credentials: Vec<u8> = intent.credentials().iter().flatten().copied().collect();
            let inserted = sqlx::query("INSERT INTO rullst_passkey_ceremony.pending (challenge,binding,kind,credentials,issued_at,expires_at) VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT(challenge) DO NOTHING")
                .bind(intent.challenge().as_slice()).bind(intent.binding().as_slice()).bind(kind_number(intent.kind())).bind(credentials).bind(op.now).bind(expires)
                .execute(&mut *op.tx).await.map_err(|_| Error::Unavailable)?;
            if inserted.rows_affected() != 1 {
                return Err(Error::Conflict);
            }
            op.until(expires)?;
            op.finish().await
        }).await
    }
    async fn consume(
        &self,
        challenge: [u8; 32],
        binding: [u8; 32],
        kind: CeremonyKind,
    ) -> Result<ConsumedCeremony, Error> {
        bounded(async {
            let mut op = self.begin().await?;
            type Row = (Vec<u8>, i16, Vec<u8>, i64, i64);
            let row: Option<Row> = sqlx::query_as("SELECT pg_catalog.substr(binding,1,33),kind,pg_catalog.substr(credentials,1,1025),issued_at,expires_at FROM rullst_passkey_ceremony.pending WHERE challenge=$1")
                .bind(challenge.as_slice()).fetch_optional(&mut *op.tx).await.map_err(|_| Error::Unavailable)?;
            let Some((stored_binding, stored_kind, credentials, issued_at, expires_at)) = row else {
                return Err(Error::Rejected);
            };
            if stored_binding.len() != 32
                || !matches!(stored_kind, 1 | 2)
                || credentials.len() > 1024
                || credentials.len() % 32 != 0
                || issued_at > op.now
                || expires_at.checked_sub(issued_at) != Some(i64::from(self.config.lifetime_seconds()))
            {
                return Err(Error::Corrupt);
            }
            if stored_kind != kind_number(kind) || stored_binding.ct_eq(&binding).unwrap_u8() != 1 {
                return Err(Error::Rejected);
            }
            let credentials = credentials.as_chunks::<32>().0.to_vec();
            let intent = CeremonyIntent::new(challenge, binding, kind, credentials)
                .map_err(|_| Error::Corrupt)?;
            let consumed = ConsumedCeremony::from_stored(intent, issued_at, expires_at)?;
            op.until(expires_at)?;
            let deleted = sqlx::query("DELETE FROM rullst_passkey_ceremony.pending WHERE challenge=$1")
                .bind(challenge.as_slice())
                .execute(&mut *op.tx)
                .await
                .map_err(|_| Error::Unavailable)?;
            if deleted.rows_affected() != 1 {
                return Err(Error::Corrupt);
            }
            op.finish().await?;
            Ok(consumed)
        }).await
    }
    async fn confirm(&self, consumed: &ConsumedCeremony) -> Result<(), Error> {
        bounded(async {
            let mut op = self.begin().await?;
            if consumed.issued_at() > op.now
                || consumed.expires_at() - consumed.issued_at()
                    != i64::from(self.config.lifetime_seconds())
            {
                return Err(Error::Corrupt);
            }
            op.until(consumed.expires_at())?;
            op.finish().await
        })
        .await
    }
}
fn kind_number(kind: CeremonyKind) -> i16 {
    match kind {
        CeremonyKind::Registration => 1,
        CeremonyKind::Authentication => 2,
    }
}

async fn bounded<T: Send>(
    operation: impl std::future::Future<Output = Result<T, Error>> + Send,
) -> Result<T, Error> {
    tokio::time::timeout(std::time::Duration::from_secs(12), operation)
        .await
        .map_err(|_| Error::UncertainCommit)?
}
