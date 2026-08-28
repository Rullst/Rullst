use std::sync::Arc;

use super::{Cache, CacheError};
use crate::security::TenantContext;

const MAX_LOGICAL_KEY_BYTES: usize = 256;

/// Cache facade permanently bound to one authenticated tenant context.
///
/// Every operation derives the backend key from the immutable tenant identity.
/// The wrapper intentionally has no global `flush`; callers may only forget
/// explicit keys inside their tenant namespace.
#[derive(Clone)]
#[non_exhaustive]
pub struct TenantCache {
    cache: Cache,
    tenant_id: String,
}

impl TenantCache {
    /// Binds a cache to a tenant already validated by authentication.
    pub fn from_context(cache: Cache, context: &TenantContext) -> Self {
        Self {
            cache,
            tenant_id: context.tenant_id.clone(),
        }
    }

    /// Returns the authenticated tenant identifier bound to this instance.
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }

    /// Returns the canonical backend key within this instance's tenant namespace.
    pub fn namespaced_key(&self, logical_key: &str) -> Result<String, CacheError> {
        validate_logical_key(logical_key)?;
        Ok(format!("tenants:{}:{logical_key}", self.tenant_id))
    }

    /// Retrieves a value only from this instance's tenant namespace.
    pub async fn get(&self, logical_key: &str) -> Result<Option<Arc<String>>, CacheError> {
        self.cache.get(&self.namespaced_key(logical_key)?).await
    }

    /// Stores a value only inside this instance's tenant namespace.
    pub async fn put(
        &self,
        logical_key: &str,
        value: &str,
        ttl_secs: Option<u64>,
    ) -> Result<(), CacheError> {
        self.cache
            .put(&self.namespaced_key(logical_key)?, value, ttl_secs)
            .await
    }

    /// Checks for a value only inside this instance's tenant namespace.
    pub async fn has(&self, logical_key: &str) -> Result<bool, CacheError> {
        self.cache.has(&self.namespaced_key(logical_key)?).await
    }

    /// Removes a value only from this instance's tenant namespace.
    pub async fn forget(&self, logical_key: &str) -> Result<(), CacheError> {
        self.cache.forget(&self.namespaced_key(logical_key)?).await
    }

    /// Retrieves a tenant-scoped value or computes and caches it.
    pub async fn remember<F, Fut>(
        &self,
        logical_key: &str,
        ttl_secs: u64,
        compute: F,
    ) -> Result<Arc<String>, CacheError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<String, CacheError>>,
    {
        self.cache
            .remember(&self.namespaced_key(logical_key)?, ttl_secs, compute)
            .await
    }
}

fn validate_logical_key(key: &str) -> Result<(), CacheError> {
    if key.is_empty() || key.len() > MAX_LOGICAL_KEY_BYTES {
        return Err(CacheError::InvalidKey(
            "key must contain 1 to 256 bytes".to_string(),
        ));
    }
    if !key.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
    }) {
        return Err(CacheError::InvalidKey(
            "key contains unsupported characters".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::security::TenantMembership;

    #[tokio::test]
    // TM-TENANT-04
    async fn identical_keys_are_isolated_by_authenticated_tenant_context() {
        let membership = TenantMembership::try_new(["school-alpha", "school-beta"])
            .expect("valid tenant membership");
        let alpha_context = membership.select("school-alpha").expect("alpha membership");
        let beta_context = membership.select("school-beta").expect("beta membership");
        let cache = Cache::memory();
        let alpha = TenantCache::from_context(cache.clone(), &alpha_context);
        let beta = TenantCache::from_context(cache, &beta_context);

        alpha
            .put("leaderboard:course:1", "alpha", Some(60))
            .await
            .expect("alpha cache write");
        beta.put("leaderboard:course:1", "beta", Some(60))
            .await
            .expect("beta cache write");

        assert_eq!(
            alpha
                .get("leaderboard:course:1")
                .await
                .expect("alpha cache read")
                .as_deref()
                .map(String::as_str),
            Some("alpha")
        );
        assert_eq!(
            beta.get("leaderboard:course:1")
                .await
                .expect("beta cache read")
                .as_deref()
                .map(String::as_str),
            Some("beta")
        );
        assert_eq!(
            alpha
                .namespaced_key("leaderboard:course:1")
                .expect("alpha key"),
            "tenants:school-alpha:leaderboard:course:1"
        );
        assert!(matches!(
            alpha.get("school beta\nsecret").await,
            Err(CacheError::InvalidKey(_))
        ));
    }
}
