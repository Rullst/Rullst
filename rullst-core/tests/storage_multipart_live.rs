#![cfg(feature = "storage-multipart")]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Owned disposable service only; run through `.github/test-storage-s3-live.sh`.
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rullst_core::{
    Storage, TenantStorage,
    security::TenantMembership,
    storage::cloud::{
        CloudCredentials, CloudStorageConfig,
        multipart::{
            AbortStatus, CompletionStatus, MultipartCheckpoint, MultipartKey, MultipartLimits,
            MultipartStorage,
        },
    },
};
use std::time::Duration;
const PART: usize = 5 * 1024 * 1024;
const OBJECT: &str = "quarantine/multipart á %?#.bin";

fn storage(r2: bool, tenant: &str) -> TenantStorage {
    let endpoint = std::env::var("RULLST_STORAGE_TEST_ENDPOINT").expect("owned fixture endpoint");
    let creds = CloudCredentials::new(
        "GK11111111111111111111111111111111",
        "2222222222222222222222222222222222222222222222222222222222222222",
    )
    .unwrap();
    let config = CloudStorageConfig::new(creds)
        .with_loopback_test_endpoint(endpoint)
        .unwrap();
    let engine = if r2 {
        Storage::r2("private-files", "1234567890abcdef1234567890abcdef")
    } else {
        Storage::s3("private-files", "auto")
    };
    let membership = TenantMembership::try_new([tenant]).unwrap();
    TenantStorage::from_context(
        engine.with_cloud_config(config).unwrap(),
        &membership.select(tenant).unwrap(),
    )
}
fn uploader(storage: &TenantStorage) -> MultipartStorage {
    storage
        .multipart(
            OBJECT,
            MultipartKey::new(URL_SAFE_NO_PAD.encode([79; 32])).unwrap(),
            MultipartLimits::new(PART as u64 * 3, PART as u32, Duration::from_secs(3600)).unwrap(),
        )
        .unwrap()
}
fn digest(bytes: &[u8]) -> [u8; 32] {
    ring::digest::digest(&ring::digest::SHA256, bytes)
        .as_ref()
        .try_into()
        .unwrap()
}

#[tokio::test]
#[ignore = "owned S3 service and checkpoint path are required"]
async fn multipart_native_prepare_restart() {
    let mut persisted = Vec::new();
    for r2 in [false, true] {
        let tenant = if r2 { "multipart-r2" } else { "multipart-s3" };
        let store = storage(r2, tenant);
        let upload = uploader(&store);
        let token = upload.begin(PART as u64 + 4).await.unwrap();
        assert!(
            uploader(&storage(r2, "another-tenant"))
                .progress(&token)
                .await
                .is_err()
        );
        let bytes = vec![if r2 { 2 } else { 1 }; PART];
        let token = upload
            .upload_part(&token, 1, &bytes, digest(&bytes))
            .await
            .unwrap();
        let independent = uploader(&storage(r2, tenant));
        let parts = independent.progress(&token).await.unwrap();
        assert!(parts[0].matches_checkpoint);
        assert!(!parts[1].matches_checkpoint);
        assert_eq!(
            independent.reconcile_completion(&token).await.unwrap(),
            CompletionStatus::Unconfirmed
        );
        persisted.push(token.expose_encoded().to_owned());
        let cancelled = independent.begin(4).await.unwrap();
        let cancelled = independent
            .upload_part(&cancelled, 1, b"stop", digest(b"stop"))
            .await
            .unwrap();
        assert_eq!(
            independent.abort(&cancelled).await.unwrap(),
            AbortStatus::Gone
        );
        assert_eq!(
            independent.abort(&cancelled).await.unwrap(),
            AbortStatus::Gone
        );
        assert!(independent.complete(&cancelled).await.is_err());
    }
    let file = std::env::var("RULLST_MULTIPART_TEST_CHECKPOINT").expect("owned checkpoint path");
    std::fs::write(file, serde_json::to_vec(&persisted).unwrap()).unwrap();
}

#[tokio::test]
#[ignore = "owned S3 service must be restarted after the prepare process exits"]
async fn multipart_native_resume_after_service_and_process_restart() {
    let file = std::env::var("RULLST_MULTIPART_TEST_CHECKPOINT").expect("owned checkpoint path");
    let records: Vec<String> = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
    assert_eq!(records.len(), 2);
    for (r2, record) in [false, true].into_iter().zip(records) {
        let tenant = if r2 { "multipart-r2" } else { "multipart-s3" };
        let store = storage(r2, tenant);
        let upload = uploader(&store);
        let token = MultipartCheckpoint::from_encoded(record).unwrap();
        assert!(upload.progress(&token).await.unwrap()[0].matches_checkpoint);
        let token = upload
            .upload_part(&token, 2, b"tail", digest(b"tail"))
            .await
            .unwrap();
        upload.complete(&token).await.unwrap();
        // Simulate losing local acknowledgement, then query via an independent client.
        assert_eq!(
            uploader(&storage(r2, tenant))
                .reconcile_completion(&token)
                .await
                .unwrap(),
            CompletionStatus::Confirmed
        );
        let body = store.get(OBJECT).await.unwrap();
        assert_eq!(body.len(), PART + 4);
        assert!(body[..PART].iter().all(|v| *v == if r2 { 2 } else { 1 }));
        assert_eq!(&body[PART..], b"tail");
        assert!(storage(r2, "another-tenant").get(OBJECT).await.is_err());
        assert_eq!(upload.abort(&token).await.unwrap(), AbortStatus::Gone);
        assert_eq!(
            store.metadata(OBJECT).await.unwrap().size_bytes,
            (PART + 4) as u64
        );
        store.delete(OBJECT).await.unwrap();
    }
}
