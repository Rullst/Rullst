use super::{
    record::{Asset, Kind, Lifecycle, Record, create_digest, random_hex},
    store::{SqliteMedia, storage},
    transaction::Operation,
};
use crate::{
    Action, Authorization, Clock, MediaError as Error, Metadata, Permission, Processing, Reference,
    Scope, VideoProvider, contracts::checked_time,
};
use std::{future::Future, time::Duration};

/// Composes current host authorization, durable ownership and one bound provider.
/// Public lifecycle operations have a 20-second total deadline. A cancelled or
/// timed-out remote mutation remains journaled; reconcile it after the lease.
pub struct MediaService<P, C> {
    pub(super) provider: P,
    pub(super) store: SqliteMedia<C>,
}
impl<P: VideoProvider, C: Clock> MediaService<P, C> {
    pub fn new(provider: P, store: SqliteMedia<C>) -> Result<Self, Error> {
        if provider.binding() != store.config.binding {
            return Err(Error::Configuration);
        }
        Ok(Self { provider, store })
    }
    /// Low-level adapter access for diagnostics and explicit offline fixtures.
    /// Calling provider methods directly does not apply application authorization.
    pub fn provider(&self) -> &P {
        &self.provider
    }
    pub async fn close(&self) {
        self.store.close().await;
    }
    pub(super) fn now(&self) -> Result<i64, Error> {
        checked_time(self.store.clock.now()?)
    }
    pub(super) async fn permit<A: Authorization>(
        &self,
        auth: &A,
        actor: &Reference,
        scope: &Scope,
        action: Action,
    ) -> Result<Permission, Error> {
        let permit = auth.check(actor, scope, action).await?;
        if permit.expires_at() <= self.now()? {
            return Err(Error::Denied);
        }
        Ok(permit)
    }
    pub(super) async fn load(&self, scope: &Scope, id: &Reference) -> Result<Asset, Error> {
        let mut tx = Operation::begin(&self.store).await?;
        let asset = tx.get(scope, id).await?.asset;
        tx.commit().await?;
        Ok(asset)
    }

    /// Idempotent local creation ID, bound to the original owner and metadata.
    /// Ambiguous provider creation is never retried as a new POST.
    pub async fn create<A: Authorization>(
        &self,
        auth: &A,
        actor: &Reference,
        scope: &Scope,
        id: &Reference,
        metadata: Metadata,
    ) -> Result<Asset, Error> {
        bounded(async {
            let permit = self.permit(auth,actor,scope,Action::Manage).await?;
            let digest = create_digest(&metadata)?;
            let mut tx = Operation::begin(&self.store).await?;
            tx.until(Some(permit.expires_at()))?;
            match tx.get(scope,id).await {
                Ok(record) => {
                    if record.asset.owner != *actor || record.create_digest != digest { return Err(Error::Conflict); }
                }
                Err(Error::NotFound) => {
                    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM media_assets").fetch_one(&mut *tx.tx).await.map_err(storage)?;
                    if count >= i64::from(self.store.config.max_assets) { return Err(Error::Capacity); }
                    let mut record = Record {
                        asset: Asset { id: id.clone(), scope: scope.clone(), owner: actor.clone(), metadata, video: None,
                            lifecycle: Lifecycle::Creating, processing: Processing::AwaitingUpload, published: false, pending: true,
                            revision: 1, updated_at: tx.now, length_seconds: 0, mp4_720p: false },
                        marker: format!("rullst-video-{}", random_hex()?), create_digest: digest, pending: None,
                        notifications: Vec::new(), last_notification: 0,
                    };
                    record.plan(Kind::Create); record.validate()?;
                    let body = serde_json::to_string(&record).map_err(|_| Error::Configuration)?;
                    sqlx::query("INSERT INTO media_assets (id,tenant,course,video,revision,body) VALUES (?,?,?,NULL,1,?)")
                        .bind(id.as_str()).bind(scope.tenant.as_str()).bind(scope.course.as_str()).bind(body)
                        .execute(&mut *tx.tx).await.map_err(storage)?;
                }
                Err(error) => return Err(error),
            }
            tx.commit().await?;
            self.drive(auth,actor,scope,id).await
        }).await
    }

    /// Resume the persisted intent after restart/cancellation/uncertain outcome.
    pub async fn reconcile<A: Authorization>(
        &self,
        auth: &A,
        actor: &Reference,
        scope: &Scope,
        id: &Reference,
    ) -> Result<Asset, Error> {
        bounded(self.drive(auth, actor, scope, id)).await
    }

    pub async fn refresh<A: Authorization>(
        &self,
        auth: &A,
        actor: &Reference,
        scope: &Scope,
        id: &Reference,
        expected_revision: i64,
    ) -> Result<Asset, Error> {
        bounded(async {
            let permit = self.permit(auth, actor, scope, Action::Manage).await?;
            self.plan(
                scope,
                id,
                expected_revision,
                Kind::Refresh,
                None,
                Some(permit.expires_at()),
            )
            .await?;
            self.drive(auth, actor, scope, id).await
        })
        .await
    }

    pub async fn update<A: Authorization>(
        &self,
        auth: &A,
        actor: &Reference,
        scope: &Scope,
        id: &Reference,
        expected_revision: i64,
        metadata: Metadata,
    ) -> Result<Asset, Error> {
        bounded(async {
            let permit = self.permit(auth, actor, scope, Action::Manage).await?;
            self.plan(
                scope,
                id,
                expected_revision,
                Kind::Update,
                Some(metadata),
                Some(permit.expires_at()),
            )
            .await?;
            self.drive(auth, actor, scope, id).await
        })
        .await
    }

    /// Stops new grants before remote deletion. Provider caches and already
    /// issued bearer capabilities may remain usable for their provider lifetime.
    pub async fn delete<A: Authorization>(
        &self,
        auth: &A,
        actor: &Reference,
        scope: &Scope,
        id: &Reference,
        expected_revision: i64,
    ) -> Result<Asset, Error> {
        bounded(async {
            let permit = self.permit(auth, actor, scope, Action::Manage).await?;
            self.plan(
                scope,
                id,
                expected_revision,
                Kind::Delete,
                None,
                Some(permit.expires_at()),
            )
            .await?;
            self.drive(auth, actor, scope, id).await
        })
        .await
    }

    pub(super) async fn plan(
        &self,
        scope: &Scope,
        id: &Reference,
        revision: i64,
        kind: Kind,
        metadata: Option<Metadata>,
        permission_until: Option<i64>,
    ) -> Result<(), Error> {
        let mut tx = Operation::begin(&self.store).await?;
        tx.until(permission_until)?;
        let mut record = tx.get(scope, id).await?;
        if revision <= 0 || record.asset.revision != revision {
            return Err(Error::Conflict);
        }
        if record.pending.is_some() {
            return Err(Error::Busy);
        }
        if record.asset.lifecycle != Lifecycle::Active {
            return Err(Error::Conflict);
        }
        if let Some(metadata) = metadata {
            record.asset.metadata = metadata;
        }
        if kind == Kind::Delete {
            record.asset.lifecycle = Lifecycle::Deleting;
            record.asset.published = false;
        }
        record.plan(kind);
        record.next(tx.now)?;
        tx.save(&record, revision).await?;
        tx.commit().await
    }
}

pub(super) async fn bounded<T>(work: impl Future<Output = Result<T, Error>>) -> Result<T, Error> {
    tokio::time::timeout(Duration::from_secs(20), work)
        .await
        .map_err(|_| Error::Uncertain)?
}
