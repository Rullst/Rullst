use rullst::security::TenantMembership;
use rullst::security_runtime::rbac::{RbacGuard, UserContext};
use rullst::storage::cloud::{CloudCredentials, CloudStorageConfig};
use rullst::{Storage, StorageError, TenantStorage};

// The host obtains these records from authenticated state and its own database.
// Bucket paths and request headers never establish ownership.
struct FileRecord {
    tenant: &'static str,
    owner: &'static str,
    key: &'static str,
}

fn authorize(storage: &Storage, user: &UserContext, file: &FileRecord) -> Option<TenantStorage> {
    RbacGuard::authorize_tenant(user, file.tenant).ok()?;
    RbacGuard::authorize_owner_or_role(user, file.owner, "file-reviewer").ok()?;
    let membership = TenantMembership::try_new([user.tenant_id()?]).ok()?;
    Some(TenantStorage::from_context(
        storage.clone(),
        &membership.select(file.tenant).ok()?,
    ))
}

#[test]
fn private_files_require_both_tenant_and_account_authorization() {
    let runtime = rullst::async_runtime::tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let storage = Storage::s3("private-files", "us-east-1")
            .with_cloud_config(CloudStorageConfig::new(
                CloudCredentials::new("mock_access", "mock_secret").unwrap(),
            ))
            .unwrap();
        let a = UserContext::new("alice", vec![])
            .try_with_tenant_id("school-a")
            .unwrap();
        let b = UserContext::new("bob", vec![])
            .try_with_tenant_id("school-a")
            .unwrap();
        let outsider_admin = UserContext::new("alice", vec!["admin".into()])
            .try_with_tenant_id("school-b")
            .unwrap();
        let file = FileRecord {
            tenant: "school-a",
            owner: "alice",
            key: "certificates/one.pdf",
        };
        let authorized = authorize(&storage, &a, &file).unwrap();
        authorized
            .put(file.key, b"authorized private certificate")
            .await
            .unwrap();
        assert!(authorize(&storage, &b, &file).is_none());
        assert!(authorize(&storage, &outsider_admin, &file).is_none());
        assert_eq!(
            authorize(&storage, &a, &file)
                .unwrap()
                .get(file.key)
                .await
                .unwrap(),
            b"authorized private certificate"
        );
        assert_eq!(authorized.metadata(file.key).await.unwrap().size_bytes, 30);
        authorized.delete(file.key).await.unwrap();
        assert!(matches!(
            authorized.get(file.key).await,
            Err(StorageError::NotFound(_))
        ));
    });
}
