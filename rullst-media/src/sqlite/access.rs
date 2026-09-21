use super::{
    record::{Asset, Kind, Lifecycle},
    service::{MediaService, bounded},
    store::storage,
    transaction::Operation,
};
use crate::{
    Action, Authorization, Clock, MediaError as Error, PlaybackGrant, PlaybackKind, Processing,
    Reference, Scope, UploadGrant, VideoProvider,
};

impl<P: VideoProvider, C: Clock> MediaService<P, C> {
    pub async fn get<A: Authorization>(
        &self,
        auth: &A,
        actor: &Reference,
        scope: &Scope,
        id: &Reference,
    ) -> Result<Asset, Error> {
        bounded(async {
            self.permit(auth, actor, scope, Action::Manage).await?;
            let asset = self.load(scope, id).await?;
            self.permit(auth, actor, scope, Action::Manage).await?;
            Ok(asset)
        })
        .await
    }

    /// Bounded local course inventory, never provider-wide listing to learners.
    pub async fn list<A: Authorization>(
        &self,
        auth: &A,
        actor: &Reference,
        scope: &Scope,
        after: Option<&Reference>,
        limit: u32,
    ) -> Result<Vec<Asset>, Error> {
        bounded(async {
            self.permit(auth,actor,scope,Action::Manage).await?;
            if limit == 0 || limit > 100 { return Err(Error::InvalidInput); }
            let mut tx = Operation::begin(&self.store).await?;
            let ids: Vec<String> = sqlx::query_scalar("SELECT id FROM media_assets WHERE tenant=? AND course=? AND id>? ORDER BY id LIMIT ?")
                .bind(scope.tenant.as_str()).bind(scope.course.as_str()).bind(after.map_or("",Reference::as_str)).bind(i64::from(limit))
                .fetch_all(&mut *tx.tx).await.map_err(storage)?;
            let mut assets = Vec::with_capacity(ids.len());
            for id in ids { assets.push(tx.get(scope,&Reference::new(id).map_err(|_| Error::Configuration)?).await?.asset); }
            tx.commit().await?;
            self.permit(auth,actor,scope,Action::Manage).await?;
            Ok(assets)
        }).await
    }

    /// Refreshes authoritative processing state before explicit publication.
    pub async fn publish<A: Authorization>(
        &self,
        auth: &A,
        actor: &Reference,
        scope: &Scope,
        id: &Reference,
        revision: i64,
    ) -> Result<Asset, Error> {
        bounded(async {
            let permit = self.permit(auth, actor, scope, Action::Manage).await?;
            self.plan(
                scope,
                id,
                revision,
                Kind::Refresh,
                None,
                Some(permit.expires_at()),
            )
            .await?;
            let current = self.drive(auth, actor, scope, id).await?;
            self.set_publication(auth, actor, scope, id, current.revision, true)
                .await
        })
        .await
    }

    /// Effective immediately for new grants, including while provider work is
    /// pending. In-flight results with the prior revision cannot restore access.
    pub async fn withdraw<A: Authorization>(
        &self,
        auth: &A,
        actor: &Reference,
        scope: &Scope,
        id: &Reference,
        revision: i64,
    ) -> Result<Asset, Error> {
        bounded(self.set_publication(auth, actor, scope, id, revision, false)).await
    }

    async fn set_publication<A: Authorization>(
        &self,
        auth: &A,
        actor: &Reference,
        scope: &Scope,
        id: &Reference,
        revision: i64,
        publish: bool,
    ) -> Result<Asset, Error> {
        let permit = self.permit(auth, actor, scope, Action::Manage).await?;
        let mut tx = Operation::begin(&self.store).await?;
        tx.until(Some(permit.expires_at()))?;
        let mut record = tx.get(scope, id).await?;
        if record.asset.revision != revision || revision <= 0 {
            return Err(Error::Conflict);
        }
        if publish
            && (record.asset.lifecycle != Lifecycle::Active
                || record.asset.processing != Processing::Ready
                || record.pending.is_some())
        {
            return Err(Error::Conflict);
        }
        if record.asset.lifecycle == Lifecycle::Deleted {
            return Err(Error::Conflict);
        }
        record.asset.published = publish;
        record.next(tx.now)?;
        tx.save(&record, revision).await?;
        tx.commit().await?;
        Ok(record.asset)
    }

    pub async fn upload<A: Authorization>(
        &self,
        auth: &A,
        actor: &Reference,
        scope: &Scope,
        id: &Reference,
        ttl: u32,
    ) -> Result<UploadGrant, Error> {
        bounded(async {
            let permit = self.permit(auth, actor, scope, Action::Manage).await?;
            let before = self.load(scope, id).await?;
            if !before.pending {
                self.plan(
                    scope,
                    id,
                    before.revision,
                    Kind::Refresh,
                    None,
                    Some(permit.expires_at()),
                )
                .await?;
            }
            let lease = self
                .claim(scope, id, Some(permit.expires_at()), Some(Kind::Refresh))
                .await?
                .ok_or(Error::Conflict)?;
            let remote = self.execute(&lease).await?;
            let permit = self.permit(auth, actor, scope, Action::Manage).await?;
            let asset = self
                .finish(lease, remote, None, Some(permit.expires_at()))
                .await?;
            if asset.lifecycle != Lifecycle::Active
                || asset.pending
                || asset.published
                || !matches!(
                    asset.processing,
                    Processing::AwaitingUpload | Processing::Failed
                )
            {
                return Err(Error::Conflict);
            }
            let permission = self.permit(auth, actor, scope, Action::Manage).await?;
            let mut tx = Operation::begin(&self.store).await?;
            tx.until(Some(permission.expires_at()))?;
            let current = tx.get(scope, id).await?;
            if current.asset.revision != asset.revision {
                return Err(Error::Conflict);
            }
            let ttl = bounded_ttl(tx.now, permission.expires_at(), ttl, 3600)?;
            let grant = self.provider.upload(
                asset.video.as_ref().ok_or(Error::Configuration)?,
                tx.now,
                ttl,
            )?;
            if grant.mode != self.store.config.binding.mode
                || grant.library != self.store.config.binding.library
                || Some(&grant.video) != asset.video.as_ref()
                || grant.expires_at > permission.expires_at()
            {
                return Err(Error::Protocol);
            }
            tx.commit().await?;
            if self.now()? >= grant.expires_at {
                return Err(Error::Expired);
            }
            Ok(grant)
        })
        .await
    }

    /// Every new playback grant refreshes provider state, rechecks entitlement
    /// and fences local revocation. No stale cached ready state on provider outage.
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
            let permit = self.permit(auth, actor, scope, Action::Play).await?;
            let before = self.load(scope, id).await?;
            if before.lifecycle != Lifecycle::Active
                || !before.published
                || before.processing != Processing::Ready
            {
                return Err(Error::Denied);
            }
            if !before.pending {
                self.plan(
                    scope,
                    id,
                    before.revision,
                    Kind::Refresh,
                    None,
                    Some(permit.expires_at()),
                )
                .await?;
            }
            let lease = self
                .claim(scope, id, Some(permit.expires_at()), Some(Kind::Refresh))
                .await?
                .ok_or(Error::Conflict)?;
            let remote = self.execute(&lease).await?;
            let permit = self.permit(auth, actor, scope, Action::Play).await?;
            let current = self
                .finish(lease, remote, None, Some(permit.expires_at()))
                .await?;
            if !current.published || current.processing != Processing::Ready {
                return Err(Error::Denied);
            }
            if kind == PlaybackKind::Mp4_720p && !current.mp4_720p {
                return Err(Error::Unsupported);
            }
            let permission = self.permit(auth, actor, scope, Action::Play).await?;
            let mut tx = Operation::begin(&self.store).await?;
            tx.until(Some(permission.expires_at()))?;
            let last = tx.get(scope, id).await?;
            if last.asset.revision != current.revision
                || !last.asset.published
                || last.asset.pending
            {
                return Err(Error::Conflict);
            }
            let ttl = bounded_ttl(tx.now, permission.expires_at(), ttl, 900)?;
            let grant = self.provider.playback(
                current.video.as_ref().ok_or(Error::Configuration)?,
                tx.now,
                ttl,
                kind,
            )?;
            if grant.mode != self.store.config.binding.mode
                || grant.expires_at > permission.expires_at()
            {
                return Err(Error::Protocol);
            }
            tx.commit().await?;
            if self.now()? >= grant.expires_at {
                return Err(Error::Expired);
            }
            Ok(grant)
        })
        .await
    }
}

fn bounded_ttl(now: i64, until: i64, requested: u32, maximum: u32) -> Result<u32, Error> {
    if requested == 0 || requested > maximum {
        return Err(Error::InvalidInput);
    }
    let available = until
        .checked_sub(now)
        .filter(|n| *n > 0)
        .ok_or(Error::Denied)?;
    Ok(requested.min(u32::try_from(available).unwrap_or(u32::MAX)))
}
