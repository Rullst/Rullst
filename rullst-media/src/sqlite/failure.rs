use super::{
    record::{Asset, Kind, Lifecycle, OperationFailure, Record},
    service::{MediaService, bounded},
    transaction::Operation,
    workflow::Lease,
};
use crate::{
    Action, Authorization, Clock, MediaError as Error, Metadata, Processing, Reference, Scope,
    VideoProvider,
};

impl<P: VideoProvider, C: Clock> MediaService<P, C> {
    /// Records a non-transient outcome under the same fence as `finish` and
    /// releases the lease. The intent stays pending but is never re-executed
    /// until the host explicitly retries or discards it.
    pub(super) async fn stop_intent(
        &self,
        lease: Lease,
        reason: OperationFailure,
        permission_until: Option<i64>,
    ) -> Result<(), Error> {
        let mut tx = Operation::begin(&self.store).await?;
        tx.until(permission_until)?;
        let previous = &lease.record.asset;
        let mut record = tx.get(&previous.scope, &previous.id).await?;
        let revision = record.asset.revision;
        let pending = record.pending.as_mut().ok_or(Error::Conflict)?;
        tx.until(Some(pending.until))?;
        if revision != previous.revision
            || pending.revision != previous.revision
            || pending.nonce.as_deref() != Some(&lease.nonce)
        {
            return Err(Error::Conflict);
        }
        pending.nonce = None;
        pending.until = 0;
        if pending.kind == Kind::Create && reason == OperationFailure::Rejected {
            // The refusal was definitive, so an explicit retry may send again.
            pending.dispatched = false;
        }
        record.asset.failure = Some(reason);
        record.next(tx.now)?;
        tx.save(&record, previous.revision).await?;
        tx.commit().await
    }

    /// Explicitly resumes a create or metadata update stopped with
    /// `Asset::failure`. A refused creation sends a new create request; an
    /// unconfirmed creation only repeats the persisted marker search and never
    /// sends another create request.
    pub async fn retry_failed<A: Authorization>(
        &self,
        auth: &A,
        actor: &Reference,
        scope: &Scope,
        id: &Reference,
        expected_revision: i64,
    ) -> Result<Asset, Error> {
        bounded(async {
            let permit = self.permit(auth, actor, scope, Action::Manage).await?;
            let mut tx = Operation::begin(&self.store).await?;
            tx.until(Some(permit.expires_at()))?;
            let mut record = stopped(tx.get(scope, id).await?, expected_revision)?.0;
            record.asset.failure = None;
            record.next(tx.now)?;
            tx.save(&record, expected_revision).await?;
            tx.commit().await?;
            self.drive(auth, actor, scope, id).await
        })
        .await
    }

    /// Abandons a stopped intent without contacting the provider.
    ///
    /// A stopped creation becomes a local tombstone: retire its creation ID.
    /// If a remote video carrying its marker exists or appears later, this
    /// store neither owns nor deletes it. A stopped metadata update keeps the
    /// requested local metadata while the remote may still hold earlier
    /// values; a missing remote video is recorded as missing and unpublished.
    pub async fn discard_failed<A: Authorization>(
        &self,
        auth: &A,
        actor: &Reference,
        scope: &Scope,
        id: &Reference,
        expected_revision: i64,
    ) -> Result<Asset, Error> {
        bounded(async {
            let permit = self.permit(auth, actor, scope, Action::Manage).await?;
            let mut tx = Operation::begin(&self.store).await?;
            tx.until(Some(permit.expires_at()))?;
            let (mut record, failure) = stopped(tx.get(scope, id).await?, expected_revision)?;
            let kind = record.pending.take().ok_or(Error::Configuration)?.kind;
            record.asset.failure = None;
            let asset = &mut record.asset;
            match (kind, failure) {
                (Kind::Create, _) => {
                    asset.lifecycle = Lifecycle::Deleted;
                    asset.processing = Processing::Missing;
                    asset.published = false;
                    asset.metadata = Metadata::new("Deleted video", "")?;
                    asset.length_seconds = 0;
                    asset.mp4_720p = false;
                    record.notifications.clear();
                }
                (Kind::Update, OperationFailure::RemoteMissing) => {
                    asset.processing = Processing::Missing;
                    asset.published = false;
                    asset.mp4_720p = false;
                }
                (Kind::Update, _) => {}
                (Kind::Refresh | Kind::Delete, _) => return Err(Error::Configuration),
            }
            record.next(tx.now)?;
            tx.save(&record, expected_revision).await?;
            tx.commit().await?;
            Ok(record.asset)
        })
        .await
    }
}

/// Accepts only a current record whose pending intent has stopped.
fn stopped(record: Record, expected_revision: i64) -> Result<(Record, OperationFailure), Error> {
    match record.asset.failure {
        Some(failure) if expected_revision > 0 && record.asset.revision == expected_revision => {
            Ok((record, failure))
        }
        _ => Err(Error::Conflict),
    }
}
