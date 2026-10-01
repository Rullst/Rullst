use std::ffi::OsStr;
use std::path::{Component, Path};

use super::{Storage, StorageDriver, StorageError, normalized_object_key};
use crate::security::TenantContext;

/// Storage facade permanently bound to one authenticated tenant context.
///
/// Object keys are placed below `tenants/<tenant_id>/`; callers cannot escape
/// that prefix through absolute paths, parent components, or backslashes. The
/// tenant identifier itself must be exactly one normal path segment: object
/// operations fail with [`StorageError::PathTraversal`] when it is empty,
/// contains `/` or `\`, or consists only of dots. On local storage, an
/// operation also fails with [`StorageError::PathTraversal`] when the
/// filesystem resolves `tenants/<tenant_id>` to a directory with a different
/// name: case-insensitive filesystems (default APFS and NTFS) would otherwise
/// give `Acme` and `acme` one directory, and Windows strips the trailing dot
/// of `acme.`. The first tenant to create such a directory keeps it. The
/// wrapper provides namespace isolation, while membership authorization and
/// backend bucket policy remain application and deployment responsibilities.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct TenantStorage {
    pub(super) storage: Storage,
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
        #[cfg(feature = "storage-s3")]
        if self.storage.cloud.is_some() {
            super::cloud::validate_key(relative_path)?;
        }
        let tenant = tenant_root_segment(&self.tenant_id)?;
        let path = normalized_object_key(relative_path)?;
        Ok(format!("tenants/{tenant}/{path}"))
    }

    /// Stores bytes below this instance's immutable tenant prefix.
    pub async fn put(&self, relative_path: &str, bytes: &[u8]) -> Result<(), StorageError> {
        let key = self.object_key(relative_path)?;
        self.confine_local_root(true).await?;
        self.storage.put(&key, bytes).await
    }

    /// Retrieves bytes only from this instance's immutable tenant prefix.
    pub async fn get(&self, relative_path: &str) -> Result<Vec<u8>, StorageError> {
        let key = self.object_key(relative_path)?;
        self.confine_local_root(false).await?;
        self.storage.get(&key).await
    }

    /// On local storage, proves that `tenants/<tenant_id>` names its own
    /// directory: the canonical path's final component must equal the tenant
    /// segment byte for byte. Case-insensitive filesystems and Windows
    /// trailing-dot stripping resolve a different spelling to an existing
    /// directory, which canonicalization reports under its real name.
    ///
    /// With `create`, the directory is created first (after rejecting symlinks
    /// below the base), so the first tenant to write owns it. Without it, a
    /// missing directory is left to the operation, which reports `NotFound`.
    pub(super) async fn confine_local_root(&self, create: bool) -> Result<(), StorageError> {
        #[cfg(feature = "storage-s3")]
        if self.storage.cloud.is_some() {
            return Ok(());
        }
        let StorageDriver::Local { base_path } = &self.storage.driver else {
            return Ok(());
        };
        let segment = tenant_root_segment(&self.tenant_id)?;
        let relative = Path::new("tenants").join(segment);
        let tenant_dir = Path::new(base_path).join(&relative);
        if create {
            tokio::fs::create_dir_all(base_path).await?;
            let canonical_base = tokio::fs::canonicalize(base_path).await?;
            super::local_paths::reject_symlink_components(&canonical_base, &relative).await?;
            tokio::fs::create_dir_all(&tenant_dir).await?;
        }
        let canonical = match tokio::fs::canonicalize(&tenant_dir).await {
            Ok(canonical) => canonical,
            Err(error) if !create && error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(());
            }
            Err(error) => return Err(StorageError::from(error)),
        };
        if canonical.file_name() != Some(OsStr::new(segment)) {
            return Err(StorageError::PathTraversal(
                "tenant directory resolves to a differently named entry".to_string(),
            ));
        }
        Ok(())
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

    #[tokio::test]
    // TM-TENANT-04
    async fn case_variant_tenants_never_share_a_local_directory() {
        let suffix = uuid::Uuid::new_v4();
        let root = std::env::temp_dir().join(format!("rullst-tenant-case-{suffix}"));
        let storage = Storage::local(root.to_string_lossy());
        let tenant = |id: &str| {
            TenantStorage::from_context(
                storage.clone(),
                &TenantContext::try_new(id).expect("valid tenant"),
            )
        };
        let acme = tenant("acme");
        acme.put("secret.txt", b"acme").await.expect("acme write");

        // A case-sensitive filesystem keeps two directories; a case-insensitive
        // one (APFS, NTFS) or Windows dot stripping must refuse the alias.
        for alias in ["Acme", "ACME", "acme."] {
            let alias = tenant(alias);
            match alias.get("secret.txt").await {
                Err(StorageError::NotFound(_) | StorageError::PathTraversal(_)) => {}
                other => panic!("alias read must fail, got {:?}", other.map(|_| ())),
            }
            match alias.put("secret.txt", b"overwrite").await {
                Ok(()) => assert_eq!(alias.get("secret.txt").await.unwrap(), b"overwrite"),
                Err(StorageError::PathTraversal(_)) => {}
                Err(other) => panic!("unexpected alias write error: {other}"),
            }
        }
        assert_eq!(acme.get("secret.txt").await.expect("acme read"), b"acme");

        std::fs::remove_dir_all(root).expect("tenant storage cleanup");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_tenant_directory_resolving_to_another_name_is_refused() {
        let suffix = uuid::Uuid::new_v4();
        let root = std::env::temp_dir().join(format!("rullst-tenant-alias-{suffix}"));
        let storage = Storage::local(root.to_string_lossy());
        let acme = TenantStorage::from_context(
            storage.clone(),
            &TenantContext::try_new("acme").expect("valid tenant"),
        );
        acme.put("secret.txt", b"acme").await.expect("acme write");
        std::os::unix::fs::symlink(root.join("tenants/acme"), root.join("tenants/other"))
            .expect("alias link");
        let other = TenantStorage::from_context(
            storage,
            &TenantContext::try_new("other").expect("valid tenant"),
        );
        assert!(matches!(
            other.confine_local_root(false).await,
            Err(StorageError::PathTraversal(_))
        ));
        assert!(matches!(
            other.get("secret.txt").await,
            Err(StorageError::PathTraversal(_))
        ));
        assert!(matches!(
            other.put("secret.txt", b"x").await,
            Err(StorageError::PathTraversal(_))
        ));
        std::fs::remove_dir_all(root).expect("tenant storage cleanup");
    }
}
