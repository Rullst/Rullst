//! JWKS caching utilities for OAuth2/OIDC providers.

use crate::client::HttpClientExt;
use std::collections::HashMap;
use std::sync::LazyLock;
use tokio::sync::RwLock;

/// Global thread-safe cache for JSON Web Key Sets (JWKS).
pub static JWKS_CACHE: LazyLock<
    RwLock<HashMap<String, std::sync::Arc<jsonwebtoken::jwk::JwkSet>>>,
> = LazyLock::new(|| RwLock::new(HashMap::new()));

/// Fetches JWKS from the specified endpoint with memory caching.
pub async fn fetch_and_cache_jwks(
    url: &str,
    client: &dyn crate::client::HttpClient,
) -> Result<std::sync::Arc<jsonwebtoken::jwk::JwkSet>, crate::error::ConnectError> {
    #[cfg(not(test))]
    #[cfg_attr(coverage_nightly, coverage(off))]
    {
        let cache = JWKS_CACHE.read().await;
        if let Some(jwks) = cache.get(url) {
            return Ok(jwks.clone());
        }
    }

    let jwks = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json::<jsonwebtoken::jwk::JwkSet>()
        .await?;

    let jwks_arc = std::sync::Arc::new(jwks);

    #[cfg(not(test))]
    #[cfg_attr(coverage_nightly, coverage(off))]
    {
        let url_str = url.to_string();
        let mut cache = JWKS_CACHE.write().await;
        cache.insert(url_str, jwks_arc.clone());
    }

    Ok(jwks_arc)
}
