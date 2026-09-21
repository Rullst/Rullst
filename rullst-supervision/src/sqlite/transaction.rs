use super::{SqliteSupervision, storage};
use crate::{Clock, Revision, StoreConfig, SupervisionError as Error, clock::checked_time};
use sqlx::{Sqlite, Transaction};

pub(super) struct Operation<'a, C> {
    pub tx: Transaction<'static, Sqlite>,
    pub now: i64,
    pub config: &'a StoreConfig,
    pub revision: i64,
    initial_revision: i64,
    previous_time: i64,
    deadline: Option<i64>,
    clock: &'a C,
}

impl<'a, C: Clock> Operation<'a, C> {
    pub async fn begin(store: &'a SqliteSupervision<C>) -> Result<Self, Error> {
        let before = checked_time(store.clock.now()?)?;
        let mut tx = store
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(storage)?;
        let rows: Vec<(i64, i64, String, i64, i64)> = sqlx::query_as(
            "SELECT id,version,substr(config,1,1025),last_now,revision FROM rullst_supervision_meta LIMIT 2",
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(storage)?;
        let (previous_time, revision) = match rows.as_slice() {
            [(1, 2, config, time, revision)]
                if config == &store.config_key() && *time >= 0 && *revision >= 0 =>
            {
                (*time, *revision)
            }
            _ => return Err(Error::Configuration),
        };
        let now = checked_time(store.clock.now()?)?;
        if now < before || now < previous_time {
            return Err(Error::Clock);
        }
        let counts: (i64,i64,i64,i64,i64,i64) = sqlx::query_as("SELECT (SELECT COUNT(*) FROM rullst_supervision_grants), (SELECT COUNT(*) FROM rullst_supervision_sessions), (SELECT COUNT(*) FROM rullst_supervision_events), (SELECT COUNT(*) FROM rullst_supervision_managed), (SELECT COUNT(*) FROM rullst_supervision_courses), (SELECT COUNT(*) FROM rullst_supervision_analysis)")
            .fetch_one(&mut *tx).await.map_err(storage)?;
        let limits = &store.config.limits;
        for (count, limit) in [
            (counts.0, limits.grants),
            (counts.1, limits.sessions),
            (counts.2, limits.events),
            (counts.3, limits.managed),
            (counts.4, limits.managed * 64),
            (counts.5, limits.sessions),
        ] {
            if !(0..=limit).contains(&count) {
                return Err(Error::Configuration);
            }
        }
        Ok(Self {
            tx,
            now,
            config: &store.config,
            revision,
            initial_revision: revision,
            previous_time,
            deadline: None,
            clock: &store.clock,
        })
    }

    pub fn next_revision(&mut self) -> Result<Revision, Error> {
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(Error::RevisionExhausted)?;
        Revision::new(self.revision)
    }

    pub fn check_revision(&self, stored: i64) -> Result<Revision, Error> {
        if stored <= 0 || stored > self.revision {
            return Err(Error::Configuration);
        }
        Revision::new(stored).map_err(|_| Error::Configuration)
    }

    pub fn until(&mut self, deadline: i64) -> Result<(), Error> {
        checked_time(deadline).map_err(|_| Error::Configuration)?;
        if deadline <= self.now {
            return Err(Error::Expired);
        }
        self.deadline = Some(self.deadline.map_or(deadline, |old| old.min(deadline)));
        Ok(())
    }

    fn check_time(&self) -> Result<i64, Error> {
        let now = checked_time(self.clock.now()?)?;
        if now < self.now || now < self.previous_time {
            return Err(Error::Clock);
        }
        if self.deadline.is_some_and(|deadline| now >= deadline) {
            return Err(Error::Expired);
        }
        Ok(now)
    }

    pub async fn finish(mut self) -> Result<(), Error> {
        self.now = self.check_time()?;
        let result = sqlx::query("UPDATE rullst_supervision_meta SET last_now=?,revision=? WHERE id=1 AND last_now=? AND revision=?")
            .bind(self.now).bind(self.revision).bind(self.previous_time).bind(self.initial_revision)
            .execute(&mut *self.tx).await.map_err(storage)?;
        if result.rows_affected() != 1 {
            return Err(Error::Configuration);
        }
        self.check_time()?;
        let clock = self.clock;
        let minimum = self.now;
        let deadline = self.deadline;
        self.tx.commit().await.map_err(|_| Error::UncertainCommit)?;
        let now = clock
            .now()
            .and_then(checked_time)
            .map_err(|_| Error::UncertainCommit)?;
        if now < minimum || deadline.is_some_and(|limit| now >= limit) {
            return Err(Error::UncertainCommit);
        }
        Ok(())
    }
}
