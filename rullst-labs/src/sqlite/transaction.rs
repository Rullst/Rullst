use super::store::{SqliteLabs, storage};
use crate::{Clock, LabError as Error, authorization::checked_time};
use sqlx::{Sqlite, Transaction};

pub(super) struct Operation<'a, C> {
    pub tx: Transaction<'static, Sqlite>,
    pub now: i64,
    previous: i64,
    deadline: Option<i64>,
    store: &'a SqliteLabs<C>,
}
impl<'a, C: Clock> Operation<'a, C> {
    pub fn narrow_deadline(&mut self, deadline: i64) -> Result<(), Error> {
        checked_time(deadline)?;
        if deadline <= self.now {
            return Err(Error::Expired);
        }
        self.deadline = Some(self.deadline.map_or(deadline, |old| old.min(deadline)));
        Ok(())
    }
    pub async fn begin(store: &'a SqliteLabs<C>, deadline: Option<i64>) -> Result<Self, Error> {
        let before = checked_time(store.clock.now()?)?;
        let mut tx = store
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(storage)?;
        let rows: Vec<(i64, i64, String, i64)> = sqlx::query_as(
            "SELECT id,version,substr(configuration,1,4097),last_now FROM labs_meta LIMIT 2",
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(storage)?;
        let previous = match rows.as_slice() {
            [(1, 1, binding, last)]
                if binding == &store.config.binding(&store.key)? && *last > 0 =>
            {
                *last
            }
            _ => return Err(Error::Configuration),
        };
        let now = checked_time(store.clock.now()?)?;
        if now < before || now < previous {
            return Err(Error::Clock);
        }
        if let Some(deadline) = deadline {
            checked_time(deadline)?;
            if deadline <= now {
                return Err(Error::Expired);
            }
        }
        let (jobs, exercises): (i64, i64) = sqlx::query_as(
            "SELECT (SELECT COUNT(*) FROM labs_jobs),(SELECT COUNT(*) FROM labs_exercises)",
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(storage)?;
        if jobs < 0
            || jobs > i64::from(store.config.max_jobs)
            || exercises < 0
            || exercises > i64::from(store.config.max_exercises)
        {
            return Err(Error::Configuration);
        }
        Ok(Self {
            tx,
            now,
            previous,
            deadline,
            store,
        })
    }
    pub async fn commit(mut self) -> Result<(), Error> {
        let now = checked_time(self.store.clock.now()?)?;
        if now < self.now {
            return Err(Error::Clock);
        }
        if self.deadline.is_some_and(|deadline| now >= deadline) {
            return Err(Error::Expired);
        }
        let result = sqlx::query("UPDATE labs_meta SET last_now=? WHERE id=1 AND last_now=?")
            .bind(now)
            .bind(self.previous)
            .execute(&mut *self.tx)
            .await
            .map_err(storage)?;
        if result.rows_affected() != 1 {
            return Err(Error::Configuration);
        }
        self.tx.commit().await.map_err(|_| Error::Uncertain)?;
        let after = self
            .store
            .clock
            .now()
            .and_then(checked_time)
            .map_err(|_| Error::Uncertain)?;
        if after < now || self.deadline.is_some_and(|deadline| after >= deadline) {
            return Err(Error::Uncertain);
        }
        Ok(())
    }
}
