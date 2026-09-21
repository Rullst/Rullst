use super::{
    record::Record,
    store::{SqliteMedia, storage},
};
use crate::{Clock, MediaError as Error, Reference, Scope, contracts::checked_time};
use sqlx::{Sqlite, Transaction};

pub(super) struct Operation<'a, C> {
    pub tx: Transaction<'static, Sqlite>,
    pub now: i64,
    previous: i64,
    deadline: Option<i64>,
    pub store: &'a SqliteMedia<C>,
}
impl<'a, C: Clock> Operation<'a, C> {
    pub async fn begin(store: &'a SqliteMedia<C>) -> Result<Self, Error> {
        let before = checked_time(store.clock.now()?)?;
        let mut tx = store
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(storage)?;
        let rows: Vec<(i64, i64, String, i64)> = sqlx::query_as(
            "SELECT id,version,substr(configuration,1,2049),last_now FROM media_meta LIMIT 2",
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(storage)?;
        let previous = match rows.as_slice() {
            [(1, 1, config, time)] if config == &store.config.key()? && *time > 0 => *time,
            _ => return Err(Error::Configuration),
        };
        let now = checked_time(store.clock.now()?)?;
        if now < before || now < previous {
            return Err(Error::Clock);
        }
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM media_assets")
            .fetch_one(&mut *tx)
            .await
            .map_err(storage)?;
        if count < 0 || count > i64::from(store.config.max_assets) {
            return Err(Error::Configuration);
        }
        Ok(Self {
            tx,
            now,
            previous,
            deadline: None,
            store,
        })
    }
    pub fn until(&mut self, deadline: Option<i64>) -> Result<(), Error> {
        if let Some(deadline) = deadline {
            checked_time(deadline)?;
            if deadline <= self.now {
                return Err(Error::Expired);
            }
            self.deadline = Some(self.deadline.map_or(deadline, |old| old.min(deadline)));
        }
        Ok(())
    }
    pub async fn get(&mut self, scope: &Scope, id: &Reference) -> Result<Record, Error> {
        let row: Option<(String,i64,Option<String>)> = sqlx::query_as("SELECT substr(body,1,32769),revision,video FROM media_assets WHERE id=? AND tenant=? AND course=?")
            .bind(id.as_str()).bind(scope.tenant.as_str()).bind(scope.course.as_str()).fetch_optional(&mut *self.tx).await.map_err(storage)?;
        let (body, revision, video) = row.ok_or(Error::NotFound)?;
        let record = Self::decode(&body)?;
        if record.asset.id != *id
            || record.asset.scope != *scope
            || record.asset.revision != revision
            || record.asset.video.as_ref().map(|v| v.as_str()) != video.as_deref()
        {
            return Err(Error::Configuration);
        }
        Ok(record)
    }
    pub fn decode(body: &str) -> Result<Record, Error> {
        if body.len() > 32768 {
            return Err(Error::Configuration);
        }
        let record: Record = serde_json::from_str(body).map_err(|_| Error::Configuration)?;
        record.validate()?;
        Ok(record)
    }
    pub async fn save(&mut self, record: &Record, old_revision: i64) -> Result<(), Error> {
        record.validate()?;
        let a = &record.asset;
        let body = serde_json::to_string(record).map_err(|_| Error::Configuration)?;
        if body.len() > 32768 {
            return Err(Error::Capacity);
        }
        let result = sqlx::query("UPDATE media_assets SET video=?,revision=?,body=? WHERE id=? AND tenant=? AND course=? AND revision=?")
            .bind(a.video.as_ref().map(|v|v.as_str())).bind(a.revision).bind(body).bind(a.id.as_str())
            .bind(a.scope.tenant.as_str()).bind(a.scope.course.as_str()).bind(old_revision)
            .execute(&mut *self.tx).await.map_err(storage)?;
        if result.rows_affected() != 1 {
            return Err(Error::Conflict);
        }
        Ok(())
    }
    pub async fn commit(mut self) -> Result<(), Error> {
        let now = checked_time(self.store.clock.now()?)?;
        if now < self.now {
            return Err(Error::Clock);
        }
        if self.deadline.is_some_and(|deadline| now >= deadline) {
            return Err(Error::Expired);
        }
        let result = sqlx::query("UPDATE media_meta SET last_now=? WHERE id=1 AND last_now=?")
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
