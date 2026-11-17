#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The lease fence of `finish` and `stop_intent` when the stored record holds
//! another lease at the lease's own revision, as an ID that was purged and
//! created again can.

use super::super::{
    record::tests::record,
    store::{SqliteMedia, StoreConfig},
};
use super::*;
use crate::{
    LibraryId,
    bunny::{BunnyConfig, BunnyCredentials, BunnyStream},
};

const T: i64 = 1_800_000_000;
const OTHER_NONCE: &str = "ffffffffffffffffffffffffffffffff";

struct Fixed;
impl Clock for Fixed {
    fn now(&self) -> Result<i64, Error> {
        Ok(T)
    }
}

/// An update lease on `record()` (planned at revision 6, leased at 7) whose
/// stored intent was then taken by another lease at revision 7.
async fn superseded(dir: &tempfile::TempDir) -> (MediaService<BunnyStream, Fixed>, Lease) {
    let provider = BunnyStream::new(
        BunnyConfig::upload_only(
            LibraryId::new(7).unwrap(),
            Reference::new("fence").unwrap(),
            BunnyCredentials::upload_only("mock_api").unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    let store = SqliteMedia::initialize(
        dir.path().join("media.sqlite"),
        StoreConfig::testing(provider.binding(), 4).unwrap(),
        Fixed,
    )
    .await
    .unwrap();
    let service = MediaService::new(provider, store).unwrap();
    let stored = record();
    sqlx::query(
        "INSERT INTO media_assets (id,tenant,course,video,revision,body) VALUES (?,?,?,?,?,?)",
    )
    .bind(stored.asset.id.as_str())
    .bind(stored.asset.scope.tenant.as_str())
    .bind(stored.asset.scope.course.as_str())
    .bind(stored.asset.video.as_ref().map(|v| v.as_str()))
    .bind(stored.asset.revision)
    .bind(serde_json::to_string(&stored).unwrap())
    .execute(&service.store.pool)
    .await
    .unwrap();
    let (scope, id) = (&stored.asset.scope, &stored.asset.id);
    service
        .plan(scope, id, 5, Kind::Update, None, None)
        .await
        .unwrap();
    let lease = service.claim(scope, id, None, None).await.unwrap().unwrap();
    assert_eq!(lease.record.asset.revision, 7);
    assert_ne!(lease.nonce, OTHER_NONCE);

    let mut tx = Operation::begin(&service.store).await.unwrap();
    let mut current = tx.get(scope, id).await.unwrap();
    current.pending.as_mut().unwrap().nonce = Some(OTHER_NONCE.to_owned());
    tx.save(&current, 7).await.unwrap();
    tx.commit().await.unwrap();
    (service, lease)
}

/// The stored record is unchanged: revision 7, still held by the other lease.
async fn assert_untouched(service: &MediaService<BunnyStream, Fixed>) {
    let stored = record();
    let mut tx = Operation::begin(&service.store).await.unwrap();
    let current = tx.get(&stored.asset.scope, &stored.asset.id).await.unwrap();
    assert_eq!(current.asset.revision, 7);
    assert_eq!(current.asset.failure, None);
    let pending = current.pending.unwrap();
    assert_eq!(pending.nonce.as_deref(), Some(OTHER_NONCE));
    assert_eq!(pending.revision, 7);
}

#[tokio::test]
async fn finish_refuses_a_record_another_lease_holds_at_the_same_revision() {
    let dir = tempfile::tempdir().unwrap();
    let (service, lease) = superseded(&dir).await;
    let video = lease.record.asset.video.clone().unwrap();
    let remote = RemoteVideo::new(
        video,
        LibraryId::new(7).unwrap(),
        "Ownership",
        "Notes",
        Processing::Ready,
        12,
        false,
    );
    assert_eq!(
        service.finish(lease, Some(remote), None, None).await.err(),
        Some(Error::Conflict)
    );
    assert_untouched(&service).await;
}

#[tokio::test]
async fn stop_intent_refuses_a_record_another_lease_holds_at_the_same_revision() {
    let dir = tempfile::tempdir().unwrap();
    let (service, lease) = superseded(&dir).await;
    assert_eq!(
        service
            .stop_intent(lease, OperationFailure::Rejected, None)
            .await,
        Err(Error::Conflict)
    );
    assert_untouched(&service).await;
}
