use super::{
    record::Lifecycle,
    service::{MediaService, bounded},
    store::storage,
    transaction::Operation,
};
use crate::{
    Action, Authorization, Clock, MediaError as Error, Reference, Scope, VideoProvider,
    contracts::checked_time,
};

impl<P: VideoProvider, C: Clock> MediaService<P, C> {
    /// Purges only confirmed-deleted local tombstones older than the supplied
    /// cutoff and at least 24 hours. The host owns retention/restore policy and
    /// must retire purged creation IDs: replay protection ends when purged.
    /// This does not erase provider backups or prove immediate CDN invalidation.
    pub async fn purge_deleted<A: Authorization>(
        &self,
        auth: &A,
        actor: &Reference,
        scope: &Scope,
        cutoff: i64,
        limit: u32,
    ) -> Result<u32, Error> {
        bounded(async {
            let permission = self.permit(auth, actor, scope, Action::Manage).await?;
            checked_time(cutoff)?;
            if limit == 0 || limit > 100 {
                return Err(Error::InvalidInput);
            }
            let mut tx = Operation::begin(&self.store).await?;
            tx.until(Some(permission.expires_at()))?;
            if cutoff > tx.now.saturating_sub(86_400) {
                return Err(Error::InvalidInput);
            }
            let ids: Vec<String> = sqlx::query_scalar(
                "SELECT id FROM media_assets WHERE tenant=? AND course=? \
                 AND json_extract(body,'$.asset.lifecycle')='Deleted' \
                 AND json_extract(body,'$.asset.updated_at')<=? ORDER BY id LIMIT ?",
            )
            .bind(scope.tenant.as_str())
            .bind(scope.course.as_str())
            .bind(cutoff)
            .bind(i64::from(limit))
            .fetch_all(&mut *tx.tx)
            .await
            .map_err(storage)?;
            let mut count = 0;
            for id in ids {
                let id = Reference::new(id).map_err(|_| Error::Configuration)?;
                let record = tx.get(scope, &id).await?;
                if record.asset.lifecycle != Lifecycle::Deleted
                    || record.pending.is_some()
                    || record.asset.updated_at > cutoff
                {
                    return Err(Error::Configuration);
                }
                let result = sqlx::query(
                    "DELETE FROM media_assets WHERE tenant=? AND course=? AND id=? AND revision=?",
                )
                .bind(scope.tenant.as_str())
                .bind(scope.course.as_str())
                .bind(id.as_str())
                .bind(record.asset.revision)
                .execute(&mut *tx.tx)
                .await
                .map_err(storage)?;
                if result.rows_affected() != 1 {
                    return Err(Error::Conflict);
                }
                count += 1;
            }
            tx.commit().await?;
            Ok(count)
        })
        .await
    }
}
