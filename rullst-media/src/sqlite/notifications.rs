use super::{
    record::Kind,
    service::{MediaService, bounded},
    store::storage,
    transaction::Operation,
};
use crate::{Clock, MediaError as Error, Reference, Scope, VerifiedNotification, VideoProvider};

impl<P: VideoProvider, C: Clock> MediaService<P, C> {
    /// Uses a verified notification only to refresh a pre-existing owned asset.
    /// Returns false for a retained duplicate. A failed attempt is not consumed.
    /// The HTTP host must rate-limit this endpoint and verify headers/body first.
    pub async fn notification(&self, event: &VerifiedNotification) -> Result<bool, Error> {
        bounded(async {
            if event.library != self.store.config.binding.library
                || event.mode != self.store.config.binding.mode
            {
                return Err(Error::Denied);
            }
            let mut tx = Operation::begin(&self.store).await?;
            let row: Option<(String, String, String)> =
                sqlx::query_as("SELECT tenant,course,id FROM media_assets WHERE video=?")
                    .bind(event.video.as_str())
                    .fetch_optional(&mut *tx.tx)
                    .await
                    .map_err(storage)?;
            let (tenant, course, id) = row.ok_or(Error::NotFound)?;
            let scope = Scope::new(tenant, course).map_err(|_| Error::Configuration)?;
            let id = Reference::new(id).map_err(|_| Error::Configuration)?;
            let record = tx.get(&scope, &id).await?;
            if record.notifications.contains(&event.digest) {
                tx.commit().await?;
                return Ok(false);
            }
            if record.last_notification != 0 && tx.now < record.last_notification.saturating_add(2)
            {
                return Err(Error::Busy);
            }
            let revision = record.asset.revision;
            tx.commit().await?;
            if record.pending.is_none() {
                self.plan(&scope, &id, revision, Kind::Refresh, None, None)
                    .await?;
            }
            let lease = self
                .claim(&scope, &id, None, Some(Kind::Refresh))
                .await?
                .ok_or(Error::Conflict)?;
            let remote = self.execute(&lease).await?;
            self.finish(lease, remote, Some(&event.digest), None)
                .await?;
            Ok(true)
        })
        .await
    }
}
