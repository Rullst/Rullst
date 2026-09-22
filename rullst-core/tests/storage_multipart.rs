#![cfg(feature = "storage-multipart")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rullst_core::{
    Storage, TenantStorage,
    security::TenantMembership,
    storage::cloud::{
        CloudCredentials, CloudStorageConfig,
        multipart::{
            AbortStatus, CompletionStatus, MultipartCheckpoint, MultipartError, MultipartKey,
            MultipartLimits,
        },
    },
};
use std::time::Duration;
const PART: u32 = 5 * 1024 * 1024;

fn config() -> CloudStorageConfig {
    CloudStorageConfig::new(CloudCredentials::new("", "").unwrap())
}
fn key() -> MultipartKey {
    MultipartKey::new(URL_SAFE_NO_PAD.encode([42; 32])).unwrap()
}
fn limits() -> MultipartLimits {
    MultipartLimits::new(u64::from(PART) * 3, PART, Duration::from_secs(3600)).unwrap()
}
fn checksum(bytes: &[u8]) -> [u8; 32] {
    ring::digest::digest(&ring::digest::SHA256, bytes)
        .as_ref()
        .try_into()
        .unwrap()
}

#[tokio::test]
async fn resume_missing_receipts_reupload_completion_and_tenant_denial() {
    let storage = Storage::s3("private-files", "auto")
        .with_cloud_config(config())
        .unwrap();
    let membership = TenantMembership::try_new(["alpha", "beta"]).unwrap();
    let a = TenantStorage::from_context(storage.clone(), &membership.select("alpha").unwrap());
    let b = TenantStorage::from_context(storage.clone(), &membership.select("beta").unwrap());
    let uploader = a.multipart("quarantine/one.bin", key(), limits()).unwrap();
    let token = uploader.begin(u64::from(PART) + 4).await.unwrap();
    assert!(matches!(
        b.multipart("quarantine/one.bin", key(), limits())
            .unwrap()
            .progress(&token)
            .await,
        Err(MultipartError::InvalidCheckpoint)
    ));
    assert!(matches!(
        a.multipart("quarantine/two.bin", key(), limits())
            .unwrap()
            .abort(&token)
            .await,
        Err(MultipartError::InvalidCheckpoint)
    ));
    assert!(matches!(
        uploader.complete(&token).await,
        Err(MultipartError::Incomplete)
    ));
    let bytes = vec![7; PART as usize];
    assert!(matches!(
        uploader.upload_part(&token, 1, &bytes, [0; 32]).await,
        Err(MultipartError::Checksum)
    ));
    assert!(matches!(
        uploader
            .upload_part(&token, 0, &bytes, checksum(&bytes))
            .await,
        Err(MultipartError::InvalidInput)
    ));
    assert!(matches!(
        uploader
            .upload_part(&token, 1, b"short", checksum(b"short"))
            .await,
        Err(MultipartError::InvalidInput)
    ));
    let accepted = uploader
        .upload_part(&token, 1, &bytes, checksum(&bytes))
        .await
        .unwrap();
    assert!(!uploader.progress(&token).await.unwrap()[0].matches_checkpoint);
    assert!(uploader.progress(&accepted).await.unwrap()[0].matches_checkpoint);
    // Re-open the facade and persisted token; provider state remains in the shared backend.
    let resumed = a.multipart("quarantine/one.bin", key(), limits()).unwrap();
    let accepted = MultipartCheckpoint::from_encoded(accepted.expose_encoded()).unwrap();
    let accepted = resumed
        .upload_part(&accepted, 2, b"tail", checksum(b"tail"))
        .await
        .unwrap();
    assert!(
        resumed
            .progress(&accepted)
            .await
            .unwrap()
            .iter()
            .all(|p| p.matches_checkpoint)
    );
    resumed.complete(&accepted).await.unwrap();
    assert_eq!(
        resumed.reconcile_completion(&accepted).await.unwrap(),
        CompletionStatus::Confirmed
    );
    let got = a.get("quarantine/one.bin").await.unwrap();
    assert_eq!(got.len(), PART as usize + 4);
    assert_eq!(&got[PART as usize..], b"tail");
    assert!(b.get("quarantine/one.bin").await.is_err());
    assert_eq!(resumed.abort(&accepted).await.unwrap(), AbortStatus::Gone);
    assert_eq!(a.get("quarantine/one.bin").await.unwrap(), got);
    a.put("quarantine/one.bin", &got).await.unwrap();
    assert_eq!(
        resumed.reconcile_completion(&accepted).await.unwrap(),
        CompletionStatus::Unconfirmed
    );
}

#[tokio::test]
async fn forged_checkpoint_wrong_key_policy_and_profile_are_rejected() {
    let storage = Storage::s3("private-files", "auto")
        .with_cloud_config(config())
        .unwrap();
    let uploader = storage.multipart("one", key(), limits()).unwrap();
    let token = uploader.begin(5).await.unwrap();
    let mut forged = token.expose_encoded().as_bytes().to_vec();
    forged[30] = if forged[30] == b'A' { b'B' } else { b'A' };
    assert!(
        uploader
            .abort(&MultipartCheckpoint::from_encoded(String::from_utf8(forged).unwrap()).unwrap())
            .await
            .is_err()
    );
    let wrong_key = MultipartKey::new(URL_SAFE_NO_PAD.encode([43; 32])).unwrap();
    assert!(
        storage
            .multipart("one", wrong_key, limits())
            .unwrap()
            .progress(&token)
            .await
            .is_err()
    );
    let changed = MultipartLimits::new(100, PART, Duration::from_secs(3600)).unwrap();
    assert!(
        storage
            .multipart("one", key(), changed)
            .unwrap()
            .progress(&token)
            .await
            .is_err()
    );
    let live = Storage::s3("private-files", "auto")
        .with_cloud_config(CloudStorageConfig::new(
            CloudCredentials::new("live", "secret").unwrap(),
        ))
        .unwrap();
    assert!(matches!(
        live.multipart("one", key(), limits())
            .unwrap()
            .progress(&token)
            .await,
        Err(MultipartError::InvalidCheckpoint)
    ));
    let fresh_mock = Storage::s3("private-files", "auto")
        .with_cloud_config(config())
        .unwrap();
    let fresh = fresh_mock.multipart("one", key(), limits()).unwrap();
    let fresh_token = fresh.begin(5).await.unwrap();
    assert!(matches!(
        fresh.abort(&token).await,
        Err(MultipartError::InvalidCheckpoint)
    ));
    assert!(fresh.progress(&fresh_token).await.is_ok());
    assert!(!format!("{token:?}").contains(token.expose_encoded()));
    assert!(!format!("{uploader:?}").contains("private-files"));
    assert_eq!(uploader.abort(&token).await.unwrap(), AbortStatus::Gone);
    assert_eq!(uploader.abort(&token).await.unwrap(), AbortStatus::Gone);
    assert!(
        uploader
            .upload_part(&token, 1, b"hello", checksum(b"hello"))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn multipart_completion_does_not_bypass_ordinary_download_budget() {
    let storage = Storage::s3("private-files", "auto")
        .with_cloud_config(config().with_limits(4, Duration::from_secs(1)).unwrap())
        .unwrap();
    let uploader = storage.multipart("one", key(), limits()).unwrap();
    let token = uploader.begin(5).await.unwrap();
    let token = uploader
        .upload_part(&token, 1, b"hello", checksum(b"hello"))
        .await
        .unwrap();
    uploader.complete(&token).await.unwrap();
    assert_eq!(storage.metadata("one").await.unwrap().size_bytes, 5);
    assert!(matches!(
        storage.get("one").await,
        Err(rullst_core::StorageError::Cloud(
            rullst_core::storage::cloud::CloudError::SizeLimit
        ))
    ));
}

#[tokio::test]
async fn explicit_mock_budget_and_limits_do_not_silently_grow() {
    assert!(MultipartLimits::new(1, PART - 1, Duration::from_secs(60)).is_err());
    assert!(MultipartLimits::new(u64::from(PART) * 257, PART, Duration::from_secs(60)).is_err());
    assert!(MultipartLimits::new(1, PART, Duration::from_secs(59)).is_err());
    assert!(
        Storage::local("unused")
            .multipart("one", key(), limits())
            .is_err()
    );
    let storage = Storage::r2("private-files", "1234567890abcdef1234567890abcdef")
        .with_cloud_config(config())
        .unwrap();
    let uploader = storage.multipart("one", key(), limits()).unwrap();
    let mut records = Vec::new();
    for _ in 0..256 {
        records.push(uploader.begin(1).await.unwrap());
    }
    assert!(uploader.begin(1).await.is_err());
    assert!(storage.put("another-object", b"x").await.is_err());
    uploader.abort(&records[0]).await.unwrap();
    assert!(uploader.begin(1).await.is_ok());
    assert!(uploader.begin(0).await.is_err());
    assert!(uploader.begin(u64::MAX).await.is_err());
}
