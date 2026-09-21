use super::{
    record::{Asset, Kind, Lifecycle, Record, random_hex},
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

    pub(super) async fn execute(&self, lease: &Lease) -> Result<Option<RemoteVideo>, Error> {
        let record = &lease.record;
        let pending = record.pending.as_ref().ok_or(Error::Configuration)?;
        let remote = match pending.kind {
            Kind::Create => {
                let video = if lease.first_dispatch {
                    self.provider.create(&record.marker).await?
                } else {
                    self.provider
                        .find_created(&record.marker)
                        .await?
                        .ok_or(Error::Uncertain)?
                };
                if video.title != record.marker {
                    return Err(Error::Protocol);
                }
                Some(video)
            }
            Kind::Update => {
                let id = record.asset.video.as_ref().ok_or(Error::Configuration)?;
                self.provider.update(id, &record.asset.metadata).await?;
                let current = self.provider.get(id).await?.ok_or(Error::Uncertain)?;
                if current.title != record.asset.metadata.title()
                    || current.description != record.asset.metadata.description()
                {
                    return Err(Error::Uncertain);
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
                    return Err(Error::Uncertain);
                }
                None
            }
        };
        if let Some(video) = &remote {
            validate_remote(video, self.store.config.binding.library)?;
            if record.asset.video.as_ref().is_some_and(|v| v != &video.id) {
                return Err(Error::Protocol);
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
            let remote = self.execute(&lease).await?;
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
