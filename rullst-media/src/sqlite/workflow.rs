use super::{
    record::{Asset, Kind, Lifecycle, OperationFailure, Record, random_hex, tombstone_digest},
    service::MediaService,
    transaction::Operation,
};
use crate::{
    Action, Authorization, Clock, MediaError as Error, Metadata, Processing, Reference,
    RemoteVideo, Scope, VideoProvider, provider::validate_remote,
};

pub(super) struct Lease {
    pub record: Record,
    pub nonce: String,
    pub first_dispatch: bool,
}

/// A provider error; `stopped` marks a non-transient create or metadata-update
/// outcome that must be recorded instead of retried after the lease.
pub(super) struct Failed {
    error: Error,
    stopped: Option<OperationFailure>,
}
impl From<Error> for Failed {
    fn from(error: Error) -> Self {
        Self {
            error,
            stopped: None,
        }
    }
}
impl From<Failed> for Error {
    fn from(failed: Failed) -> Self {
        failed.error
    }
}
fn stop(error: Error, reason: OperationFailure) -> Failed {
    Failed {
        error,
        stopped: Some(reason),
    }
}
impl<P: VideoProvider, C: Clock> MediaService<P, C> {
    pub(super) async fn claim(
        &self,
        scope: &Scope,
        id: &Reference,
        permission_until: Option<i64>,
        expected_kind: Option<Kind>,
    ) -> Result<Option<Lease>, Error> {
        let mut tx = Operation::begin(&self.store).await?;
        tx.until(permission_until)?;
        let mut record = tx.get(scope, id).await?;
        let revision = record.asset.revision;
        let Some(pending) = &mut record.pending else {
            tx.commit().await?;
            return Ok(None);
        };
        if record.asset.failure.is_some() {
            return Err(Error::Conflict);
        }
        if expected_kind.is_some_and(|kind| pending.kind != kind) {
            return Err(Error::Busy);
        }
        if pending.nonce.is_some() && pending.until > tx.now {
            return Err(Error::Busy);
        }
        let nonce = random_hex()?;
        let first_dispatch = !pending.dispatched;
        pending.dispatched = true;
        pending.nonce = Some(nonce.clone());
        pending.until = tx.now.checked_add(45).ok_or(Error::Clock)?;
        pending.revision = revision.checked_add(1).ok_or(Error::Capacity)?;
        record.next(tx.now)?;
        tx.save(&record, revision).await?;
        tx.commit().await?;
        Ok(Some(Lease {
            record,
            nonce,
            first_dispatch,
        }))
    }

    pub(super) async fn execute(&self, lease: &Lease) -> Result<Option<RemoteVideo>, Failed> {
        let record = &lease.record;
        let pending = record.pending.as_ref().ok_or(Error::Configuration)?;
        let remote = match pending.kind {
            Kind::Create => {
                let video = if lease.first_dispatch {
                    // Only a definitive refusal proves that nothing was created.
                    self.provider
                        .create(&record.marker)
                        .await
                        .map_err(|error| match error {
                            Error::Rejected | Error::NotFound => {
                                stop(error, OperationFailure::Rejected)
                            }
                            error => error.into(),
                        })?
                } else {
                    match self.provider.find_created(&record.marker).await {
                        Ok(Some(video)) => video,
                        Ok(None) => {
                            return Err(stop(
                                Error::Uncertain,
                                OperationFailure::CreationUnconfirmed,
                            ));
                        }
                        Err(Error::Conflict) => {
                            return Err(stop(
                                Error::Conflict,
                                OperationFailure::CreationUnconfirmed,
                            ));
                        }
                        Err(error) => return Err(error.into()),
                    }
                };
                if video.title != record.marker {
                    return Err(Error::Protocol.into());
                }
                Some(video)
            }
            Kind::Update => {
                let id = record.asset.video.as_ref().ok_or(Error::Configuration)?;
                let metadata = &record.asset.metadata;
                self.provider
                    .update(id, metadata)
                    .await
                    .map_err(|error| match error {
                        Error::Rejected => stop(error, OperationFailure::Rejected),
                        Error::NotFound => stop(error, OperationFailure::RemoteMissing),
                        Error::Capacity => stop(error, OperationFailure::TagCapacity),
                        error => error.into(),
                    })?;
                let current = self
                    .provider
                    .get(id)
                    .await?
                    .ok_or_else(|| stop(Error::NotFound, OperationFailure::RemoteMissing))?;
                if current.title != metadata.title()
                    || current.description != metadata.description()
                {
                    return Err(stop(
                        Error::Uncertain,
                        OperationFailure::VerificationMismatch,
                    ));
                }
                Some(current)
            }
            Kind::Refresh => {
                self.provider
                    .get(record.asset.video.as_ref().ok_or(Error::Configuration)?)
                    .await?
            }
            Kind::Delete => {
                let id = record.asset.video.as_ref().ok_or(Error::Configuration)?;
                self.provider.delete(id).await?;
                if self.provider.get(id).await?.is_some() {
                    return Err(Error::Uncertain.into());
                }
                None
            }
        };
        if let Some(video) = &remote {
            validate_remote(video, self.store.config.binding.library)?;
            if record.asset.video.as_ref().is_some_and(|v| v != &video.id) {
                return Err(Error::Protocol.into());
            }
        }
        Ok(remote)
    }

    pub(super) async fn finish(
        &self,
        lease: Lease,
        remote: Option<RemoteVideo>,
        notification: Option<&str>,
        permission_until: Option<i64>,
    ) -> Result<Asset, Error> {
        let mut tx = Operation::begin(&self.store).await?;
        tx.until(permission_until)?;
        let previous = &lease.record.asset;
        let mut record = tx.get(&previous.scope, &previous.id).await?;
        let pending = record.pending.as_ref().ok_or(Error::Conflict)?;
        let kind = pending.kind;
        let deadline = pending.until;
        tx.until(Some(deadline))?;
        if record.asset.revision != previous.revision
            || pending.revision != previous.revision
            || pending.nonce.as_deref() != Some(&lease.nonce)
            || deadline <= tx.now
        {
            return Err(Error::Conflict);
        }
        record.pending = None;
        if let Some(remote) = remote {
            record.asset.video = Some(remote.id);
            record.asset.processing = remote.processing;
            record.asset.length_seconds = remote.length_seconds;
            record.asset.mp4_720p = remote.mp4_720p;
            if remote.processing != Processing::Ready {
                record.asset.published = false;
            }
            if kind == Kind::Create {
                record.asset.lifecycle = Lifecycle::Active;
                record.plan(Kind::Update);
            }
        } else {
            if kind == Kind::Create || kind == Kind::Update {
                return Err(Error::Protocol);
            }
            record.asset.processing = Processing::Missing;
            record.asset.mp4_720p = false;
            record.asset.published = false;
            if kind == Kind::Delete {
                record.asset.lifecycle = Lifecycle::Deleted;
                record.asset.metadata = Metadata::new("Deleted video", "")?;
                record.create_digest = tombstone_digest();
                record.asset.length_seconds = 0;
                record.notifications.clear();
            }
        }
        if let Some(digest) = notification {
            record.notifications.push(digest.into());
            if record.notifications.len() > 32 {
                record.notifications.remove(0);
            }
            record.last_notification = tx.now;
        }
        record.next(tx.now)?;
        tx.save(&record, previous.revision).await?;
        if self.now()? >= deadline {
            return Err(Error::Expired);
        }
        tx.commit().await?;
        if self.now()? >= deadline {
            return Err(Error::Uncertain);
        }
        Ok(record.asset)
    }

    pub(super) async fn drive<A: Authorization>(
        &self,
        auth: &A,
        actor: &Reference,
        scope: &Scope,
        id: &Reference,
    ) -> Result<Asset, Error> {
        // A fresh create has two intents: bind its remote identity, then metadata.
        for _ in 0..2 {
            let permit = self.permit(auth, actor, scope, Action::Manage).await?;
            let Some(lease) = self
                .claim(scope, id, Some(permit.expires_at()), None)
                .await?
            else {
                self.permit(auth, actor, scope, Action::Manage).await?;
                return self.load(scope, id).await;
            };
            let remote = match self.execute(&lease).await {
                Ok(remote) => remote,
                Err(Failed {
                    error,
                    stopped: Some(reason),
                }) => {
                    let permit = self.permit(auth, actor, scope, Action::Manage).await?;
                    self.stop_intent(lease, reason, Some(permit.expires_at()))
                        .await?;
                    return Err(error);
                }
                Err(failed) => return Err(failed.into()),
            };
            let permit = self.permit(auth, actor, scope, Action::Manage).await?;
            let asset = self
                .finish(lease, remote, None, Some(permit.expires_at()))
                .await?;
            if !asset.pending {
                return Ok(asset);
            }
        }
        Err(Error::Uncertain)
    }
}
