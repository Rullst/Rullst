use std::path::{Component, Path};

use super::{Storage, StorageError, normalized_object_key};
use crate::security::TenantContext;

/// Storage facade permanently bound to one authenticated tenant context.
///
/// Object keys are placed below `tenants/<tenant_id>/`; callers cannot escape
/// that prefix through absolute paths, parent components, or backslashes. The
/// tenant identifier itself must be exactly one normal path segment: object
/// operations fail with [`StorageError::PathTraversal`] when it is empty,
/// contains `/` or `\`, or consists only of dots. The wrapper provides
/// namespace isolation, while membership authorization and backend bucket
/// policy remain application and deployment responsibilities.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct TenantStorage {
    storage: Storage,
    tenant_id: String,
}

impl TenantStorage {
    /// Binds a storage engine to a tenant already validated by authentication.
    pub fn from_context(storage: Storage, context: &TenantContext) -> Self {
        Self {
            storage,
            tenant_id: context.tenant_id.clone(),
        }
    }

    /// Returns the authenticated tenant identifier bound to this instance.
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }

    /// Returns the backend object key confined below the tenant namespace.
    pub fn object_key(&self, relative_path: &str) -> Result<String, StorageError> {
        let tenant = tenant_root_segment(&self.tenant_id)?;
        let path = normalized_object_key(relative_path)?;
        Ok(format!("tenants/{tenant}/{path}"))
    }

    /// Stores bytes below this instance's immutable tenant prefix.
    pub async fn put(&self, relative_path: &str, bytes: &[u8]) -> Result<(), StorageError> {
        self.storage
            .put(&self.object_key(relative_path)?, bytes)
            .await
    }

    /// Retrieves bytes only from this instance's immutable tenant prefix.
    pub async fn get(&self, relative_path: &str) -> Result<Vec<u8>, StorageError> {
        self.storage.get(&self.object_key(relative_path)?).await
    }

    /// Resolves a URL only after applying this instance's tenant prefix.
    pub fn url(&self, relative_path: &str) -> Result<String, StorageError> {
        self.storage.url(&self.object_key(relative_path)?)
    }
}

/// Re-checks that the tenant identifier is a single normal path segment.
///
/// [`TenantContext::tenant_id`] is a public field, so the storage root must not
/// rely on construction-time validation alone: `.` would otherwise collapse
/// `tenants/./<path>` into another tenant's root.
fn tenant_root_segment(tenant_id: &str) -> Result<&str, StorageError> {
    let mut components = Path::new(tenant_id).components();
    let single_normal = matches!(
        (components.next(), components.next()),
        (Some(Component::Normal(segment)), None) if segment.to_str() == Some(tenant_id)
    );
    if !single_normal
        || tenant_id.contains(['/', '\\', '\0'])
        || tenant_id.bytes().all(|byte| byte == b'.')
    {
        return Err(StorageError::PathTraversal(
            "tenant identifier is not a single path segment".to_string(),
        ));
    }
    Ok(tenant_id)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::security::TenantMembership;

    #[tokio::test]
    // TM-TENANT-04
    async fn identical_keys_are_isolated_by_authenticated_tenant_context() {
        let suffix = uuid::Uuid::new_v4();
        let root = std::env::temp_dir().join(format!("rullst-tenant-storage-{suffix}"));
        let membership = TenantMembership::try_new(["school-alpha", "school-beta"])
            .expect("valid tenant membership");
        let alpha_context = membership.select("school-alpha").expect("alpha membership");
        let beta_context = membership.select("school-beta").expect("beta membership");
        let storage = Storage::local(root.to_string_lossy());
        let alpha = TenantStorage::from_context(storage.clone(), &alpha_context);
        let beta = TenantStorage::from_context(storage, &beta_context);

        alpha
            .put("courses/1/lesson.txt", b"alpha")
            .await
            .expect("alpha write");
        beta.put("courses/1/lesson.txt", b"beta")
            .await
            .expect("beta write");

        assert_eq!(
            alpha.get("courses/1/lesson.txt").await.expect("alpha read"),
            b"alpha"
        );
        assert_eq!(
            beta.get("courses/1/lesson.txt").await.expect("beta read"),
            b"beta"
        );
        assert_eq!(
            alpha.object_key("courses/1/lesson.txt").expect("alpha key"),
            "tenants/school-alpha/courses/1/lesson.txt"
        );
        assert!(matches!(
            alpha.get("../school-beta/secret.txt").await,
            Err(StorageError::PathTraversal(_))
        ));

        std::fs::remove_dir_all(root).expect("tenant storage cleanup");
    }

    #[tokio::test]
    // TM-TENANT-04
    async fn tenant_ids_that_are_not_one_path_segment_are_rejected() {
        let suffix = uuid::Uuid::new_v4();
        let root = std::env::temp_dir().join(format!("rullst-tenant-segment-{suffix}"));
        let storage = Storage::local(root.to_string_lossy());
        let acme_context = TenantContext::try_new("acme").expect("valid tenant");
        let acme = TenantStorage::from_context(storage.clone(), &acme_context);
        acme.put("secret.txt", b"acme").await.expect("acme write");

        // `TenantContext::tenant_id` is public, so it can change after validation.
        for tenant_id in ["", ".", "..", "...", "acme/x", "acme\\x", "/acme", "acme/"] {
            let mut context = acme_context.clone();
            context.tenant_id = tenant_id.to_string();
            let tenant = TenantStorage::from_context(storage.clone(), &context);
            assert!(
                matches!(
                    tenant.object_key("acme/secret.txt"),
                    Err(StorageError::PathTraversal(_))
                ),
                "{tenant_id:?}"
            );
            assert!(matches!(
                tenant.get("acme/secret.txt").await,
                Err(StorageError::PathTraversal(_))
            ));
            assert!(matches!(
                tenant.put("acme/secret.txt", b"overwrite").await,
                Err(StorageError::PathTraversal(_))
            ));
        }
        assert_eq!(acme.get("secret.txt").await.expect("acme read"), b"acme");

        assert_eq!(
            acme.object_key("courses/1/lesson.txt").expect("acme key"),
            "tenants/acme/courses/1/lesson.txt"
        );
        let colon = TenantStorage::from_context(
            storage,
            &TenantContext::try_new("acme:prod").expect("valid tenant"),
        );
        assert_eq!(
            colon.object_key("lesson.txt").expect("colon key"),
            "tenants/acme:prod/lesson.txt"
        );

        std::fs::remove_dir_all(root).expect("tenant storage cleanup");
    }
}
