use super::{
    access::bounded_ttl,
    record::{Asset, Lifecycle, Record},
    service::{MediaService, bounded},
    transaction::Operation,
};
use crate::{
    Action, Authorization, Clock, MediaError as Error, PlaybackGrant, PlaybackKind, Processing,
    Reference, RemoteVideo, Scope, VideoId, VideoProvider, provider::validate_remote,
};

impl<P: VideoProvider, C: Clock> MediaService<P, C> {
    /// Every new playback grant rechecks entitlement, reads current provider
    /// readiness and fences local withdrawal/deletion in the transaction that
    /// issues it. No stale cached ready state is used on provider outage.
    ///
    /// Playback is a read. It takes no mutation lease, so concurrent viewers
    /// never serialize, and a failed, timed-out or dropped request leaves no
    /// durable intent behind. A changed observation is recorded only if the
    /// revision is unchanged since the initial load and no live lease is held;
    /// a video that is no longer ready withdraws publication.
    pub async fn playback<A: Authorization>(
        &self,
        auth: &A,
        actor: &Reference,
        scope: &Scope,
        id: &Reference,
        ttl: u32,
        kind: PlaybackKind,
    ) -> Result<PlaybackGrant, Error> {
        bounded(async {
            self.permit(auth, actor, scope, Action::Play).await?;
            let before = self.load(scope, id).await?;
            if !grantable(&before) {
                return Err(Error::Denied);
            }
            let video = before.video.clone().ok_or(Error::Configuration)?;
            let remote = self.provider.get(&video).await?;
            if let Some(remote) = &remote {
                validate_remote(remote, self.store.config.binding.library)?;
                if remote.id != video {
                    return Err(Error::Protocol);
                }
            }
            let permission = self.permit(auth, actor, scope, Action::Play).await?;
            let mut tx = Operation::begin(&self.store).await?;
            tx.until(Some(permission.expires_at()))?;
            let mut record = tx.get(scope, id).await?;
            // Withdrawal or deletion committed during the read always wins.
            if !grantable(&record.asset) || record.asset.video.as_ref() != Some(&video) {
                return Err(Error::Conflict);
            }
            let leased = record
                .pending
                .as_ref()
                .is_some_and(|p| p.nonce.is_some() && p.until > tx.now);
            if record.asset.revision == before.revision
                && !leased
                && observe(&mut record, remote.as_ref())
            {
                record.next(tx.now)?;
                tx.save(&record, before.revision).await?;
            }
            let result = self.grant(&video, remote, kind, ttl, tx.now, permission.expires_at());
            tx.commit().await?;
            let grant = result?;
            if self.now()? >= grant.expires_at {
                return Err(Error::Expired);
            }
            Ok(grant)
        })
        .await
    }

    fn grant(
        &self,
        video: &VideoId,
        remote: Option<RemoteVideo>,
        kind: PlaybackKind,
        ttl: u32,
        now: i64,
        until: i64,
    ) -> Result<PlaybackGrant, Error> {
        let remote = remote
            .filter(|remote| remote.processing == Processing::Ready)
            .ok_or(Error::Denied)?;
        if kind == PlaybackKind::Mp4_720p && !remote.mp4_720p {
            return Err(Error::Unsupported);
        }
        let ttl = bounded_ttl(now, until, ttl, 900)?;
        let grant = self.provider.playback(video, now, ttl, kind)?;
        if grant.mode != self.store.config.binding.mode || grant.expires_at > until {
            return Err(Error::Protocol);
        }
        Ok(grant)
    }
}

fn grantable(asset: &Asset) -> bool {
    asset.lifecycle == Lifecycle::Active && asset.published && asset.processing == Processing::Ready
}

/// Applies an authoritative read; returns whether any recorded field changed.
fn observe(record: &mut Record, remote: Option<&RemoteVideo>) -> bool {
    let asset = &mut record.asset;
    let previous = (
        asset.processing,
        asset.length_seconds,
        asset.mp4_720p,
        asset.published,
    );
    match remote {
        Some(remote) => {
            asset.processing = remote.processing;
            asset.length_seconds = remote.length_seconds;
            asset.mp4_720p = remote.mp4_720p;
        }
        None => {
            asset.processing = Processing::Missing;
            asset.mp4_720p = false;
        }
    }
    if asset.processing != Processing::Ready {
        asset.published = false;
    }
    previous
        != (
            asset.processing,
            asset.length_seconds,
            asset.mp4_720p,
            asset.published,
        )
}
