#![allow(clippy::unwrap_used, clippy::expect_used)]

use rullst_core::{Storage, StorageError, TenantStorage, security::TenantMembership};

#[tokio::test]
async fn metadata_and_deletion_work_without_a_cloud_feature() {
    let root = std::env::temp_dir().join(format!("rullst-local-facade-{}", uuid::Uuid::new_v4()));
    let membership = TenantMembership::try_new(["school"]).unwrap();
    let storage = TenantStorage::from_context(
        Storage::local(root.to_string_lossy()),
        &membership.select("school").unwrap(),
    );
    storage.put("file", b"local bytes").await.unwrap();
    let metadata = storage.metadata("file").await.unwrap();
    assert_eq!(metadata.size_bytes, 11);
    assert_eq!(metadata.etag, None);
    storage.delete("file").await.unwrap();
    assert!(matches!(
        storage.metadata("file").await,
        Err(StorageError::NotFound(_))
    ));
    assert!(storage.delete("../outside").await.is_err());
    tokio::fs::remove_dir_all(root).await.unwrap();
}
