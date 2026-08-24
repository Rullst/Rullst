//! Bounded JWKS caching with rotation-aware refresh and safe stale fallback.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};

use jsonwebtoken::jwk::JwkSet;
use tokio::sync::RwLock;

use crate::client::{HttpClient, HttpClientExt};
use crate::error::ConnectError;

const DEFAULT_TTL: Duration = Duration::from_secs(15 * 60);
const DEFAULT_MAX_STALE: Duration = Duration::from_secs(24 * 60 * 60);

/// Freshness and stale-on-error bounds for a JWKS cache.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct JwksCachePolicy {
    ttl: Duration,
    max_stale: Duration,
}

impl JwksCachePolicy {
    /// Creates a policy. `max_stale` is the maximum total age of a cached set.
    pub fn new(ttl: Duration, max_stale: Duration) -> Result<Self, ConnectError> {
        if max_stale < ttl {
            return Err(ConnectError::InvalidConfiguration {
                field: "jwks_max_stale",
                reason: "must be greater than or equal to the JWKS TTL".to_string(),
            });
        }
        Ok(Self { ttl, max_stale })
    }

    /// Returns the configured freshness lifetime.
    pub fn ttl(self) -> Duration {
        self.ttl
    }

    /// Returns the maximum age accepted only when refresh fails.
    pub fn max_stale(self) -> Duration {
        self.max_stale
    }
}

impl Default for JwksCachePolicy {
    fn default() -> Self {
        Self {
            ttl: DEFAULT_TTL,
            max_stale: DEFAULT_MAX_STALE,
        }
    }
}

#[derive(Clone)]
struct CacheEntry {
    keys: Arc<JwkSet>,
    fetched_at: Instant,
}

/// An isolated JWKS cache. Providers own a cache so injected clients cannot
/// accidentally share trust material with another provider instance.
#[derive(Clone)]
pub struct JwksCache {
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
    policy: JwksCachePolicy,
}

impl JwksCache {
    /// Creates an empty cache with the supplied freshness policy.
    pub fn new(policy: JwksCachePolicy) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            policy,
        }
    }

    /// Fetches a key set, refreshing entries after their TTL.
    pub async fn get(
        &self,
        url: &str,
        client: &dyn HttpClient,
    ) -> Result<Arc<JwkSet>, ConnectError> {
        let cached = self.cached(url).await;
        if let Some(entry) = cached.as_ref()
            && age(entry) <= self.policy.ttl
        {
            return Ok(entry.keys.clone());
        }

        match fetch_remote(url, client).await {
            Ok(keys) => {
                self.store(url, keys.clone()).await;
                Ok(keys)
            }
            Err(error) => {
                if let Some(entry) = cached
                    && age(&entry) <= self.policy.max_stale
                {
                    tracing::warn!(url, "JWKS refresh failed; using bounded stale key set");
                    return Ok(entry.keys);
                }
                Err(error)
            }
        }
    }

    /// Returns a set containing `kid`. A missing key forces one immediate
    /// refresh even while the cached set is fresh, which supports key rotation.
    pub async fn get_for_kid(
        &self,
        url: &str,
        kid: &str,
        client: &dyn HttpClient,
    ) -> Result<Arc<JwkSet>, ConnectError> {
        if kid.trim().is_empty() {
            return Err(ConnectError::JwkNotFound("<empty>".to_string()));
        }

        let cached = self.cached(url).await;
        if let Some(entry) = cached.as_ref()
            && age(entry) <= self.policy.ttl
            && entry.keys.find(kid).is_some()
        {
            return Ok(entry.keys.clone());
        }

        match fetch_remote(url, client).await {
            Ok(keys) => {
                self.store(url, keys.clone()).await;
                if keys.find(kid).is_some() {
                    Ok(keys)
                } else {
                    Err(ConnectError::JwkNotFound(kid.to_string()))
                }
            }
            Err(error) => {
                if let Some(entry) = cached
                    && age(&entry) <= self.policy.max_stale
                    && entry.keys.find(kid).is_some()
                {
                    tracing::warn!(
                        url,
                        kid,
                        "JWKS refresh failed; using a bounded stale matching key"
                    );
                    return Ok(entry.keys);
                }
                Err(error)
            }
        }
    }

    /// Removes all cached sets owned by this cache instance.
    pub async fn clear(&self) {
        self.entries.write().await.clear();
    }

    async fn cached(&self, url: &str) -> Option<CacheEntry> {
        self.entries.read().await.get(url).cloned()
    }

    async fn store(&self, url: &str, keys: Arc<JwkSet>) {
        self.entries.write().await.insert(
            url.to_string(),
            CacheEntry {
                keys,
                fetched_at: Instant::now(),
            },
        );
    }
}

impl Default for JwksCache {
    fn default() -> Self {
        Self::new(JwksCachePolicy::default())
    }
}

fn age(entry: &CacheEntry) -> Duration {
    Instant::now().saturating_duration_since(entry.fetched_at)
}

async fn fetch_remote(url: &str, client: &dyn HttpClient) -> Result<Arc<JwkSet>, ConnectError> {
    let validated_url = crate::configuration::validate_jwks_url(url)?;
    let keys = client
        .get(validated_url.to_string())
        .send()
        .await?
        .error_for_status()?
        .json::<JwkSet>()
        .await?;
    Ok(Arc::new(keys))
}

/// Legacy raw cache retained for source compatibility. Verification no longer
/// consumes entries from this map because they do not carry freshness metadata.
#[deprecated(
    since = "12.0.0",
    note = "use JwksCache; raw entries cannot satisfy TTL or rotation guarantees"
)]
pub static JWKS_CACHE: LazyLock<RwLock<HashMap<String, Arc<JwkSet>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

static PROCESS_JWKS_CACHE: LazyLock<JwksCache> = LazyLock::new(JwksCache::default);

/// Fetches JWKS through the compatibility cache.
pub async fn fetch_and_cache_jwks(
    url: &str,
    client: &dyn HttpClient,
) -> Result<Arc<JwkSet>, ConnectError> {
    PROCESS_JWKS_CACHE.get(url, client).await
}

/// Fetches JWKS and forces refresh when `kid` is absent from a fresh cache.
pub async fn fetch_and_cache_jwks_for_kid(
    url: &str,
    kid: &str,
    client: &dyn HttpClient,
) -> Result<Arc<JwkSet>, ConnectError> {
    PROCESS_JWKS_CACHE.get_for_kid(url, kid, client).await
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use async_trait::async_trait;
    use serde_json::json;

    use super::*;
    use crate::client::{HttpRequest, HttpResponse};

    struct SequenceClient {
        calls: AtomicUsize,
        responses: Vec<Result<serde_json::Value, &'static str>>,
    }

    #[async_trait]
    impl HttpClient for SequenceClient {
        async fn execute(&self, _req: HttpRequest) -> Result<HttpResponse, ConnectError> {
            let index = self.calls.fetch_add(1, Ordering::SeqCst);
            match self.responses.get(index).or_else(|| self.responses.last()) {
                Some(Ok(body)) => Ok(HttpResponse {
                    status: 200,
                    body: body.clone(),
                }),
                Some(Err(message)) => Err(ConnectError::Reqwest((*message).to_string())),
                None => Err(ConnectError::Reqwest("no response".to_string())),
            }
        }
    }

    fn jwks(kid: &str) -> serde_json::Value {
        json!({
            "keys": [{
                "kty": "RSA",
                "kid": kid,
                "use": "sig",
                "alg": "RS256",
                "n": "sXchDaQebHnPiGvyDO5R",
                "e": "AQAB"
            }]
        })
    }

    #[tokio::test]
    async fn unknown_kid_forces_refresh() {
        let policy = JwksCachePolicy::new(Duration::from_secs(60), Duration::from_secs(120))
            .expect("valid policy");
        let cache = JwksCache::new(policy);
        let client = SequenceClient {
            calls: AtomicUsize::new(0),
            responses: vec![Ok(jwks("old")), Ok(jwks("new"))],
        };

        cache
            .get_for_kid("https://issuer.example/jwks", "old", &client)
            .await
            .expect("old key");
        cache
            .get_for_kid("https://issuer.example/jwks", "new", &client)
            .await
            .expect("rotated key");
        assert_eq!(client.calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn stale_fallback_never_accepts_an_unknown_kid() {
        let policy =
            JwksCachePolicy::new(Duration::ZERO, Duration::from_secs(60)).expect("valid policy");
        let cache = JwksCache::new(policy);
        let client = SequenceClient {
            calls: AtomicUsize::new(0),
            responses: vec![Ok(jwks("known")), Err("offline")],
        };

        cache
            .get_for_kid("https://issuer.example/jwks", "known", &client)
            .await
            .expect("initial key");
        let error = cache
            .get_for_kid("https://issuer.example/jwks", "unknown", &client)
            .await
            .expect_err("unknown stale key must be rejected");
        assert!(matches!(error, ConnectError::Reqwest(_)));
    }

    #[tokio::test]
    async fn expired_entries_are_refreshed_and_known_keys_can_be_bounded_stale() {
        let policy =
            JwksCachePolicy::new(Duration::ZERO, Duration::from_secs(60)).expect("valid policy");
        let rotating_cache = JwksCache::new(policy);
        let rotating_client = SequenceClient {
            calls: AtomicUsize::new(0),
            responses: vec![Ok(jwks("old")), Ok(jwks("new"))],
        };

        let old = rotating_cache
            .get("https://issuer.example/rotating-jwks", &rotating_client)
            .await
            .expect("old set");
        assert!(old.find("old").is_some());
        let new = rotating_cache
            .get("https://issuer.example/rotating-jwks", &rotating_client)
            .await
            .expect("refreshed set");
        assert!(new.find("new").is_some());

        let stale_cache = JwksCache::new(policy);
        let stale_client = SequenceClient {
            calls: AtomicUsize::new(0),
            responses: vec![Ok(jwks("known")), Err("offline")],
        };
        stale_cache
            .get_for_kid("https://issuer.example/stale-jwks", "known", &stale_client)
            .await
            .expect("initial key");
        let stale = stale_cache
            .get_for_kid("https://issuer.example/stale-jwks", "known", &stale_client)
            .await
            .expect("bounded stale matching key");
        assert!(stale.find("known").is_some());
    }

    #[tokio::test]
    async fn keys_older_than_the_stale_bound_are_rejected_on_refresh_error() {
        let policy =
            JwksCachePolicy::new(Duration::ZERO, Duration::from_secs(1)).expect("valid policy");
        let cache = JwksCache::new(policy);
        let keys: JwkSet = serde_json::from_value(jwks("known")).expect("valid JWKS");
        cache.entries.write().await.insert(
            "https://issuer.example/expired-jwks".to_string(),
            CacheEntry {
                keys: Arc::new(keys),
                fetched_at: Instant::now()
                    .checked_sub(Duration::from_secs(2))
                    .expect("test instant supports a two-second offset"),
            },
        );
        let client = SequenceClient {
            calls: AtomicUsize::new(0),
            responses: vec![Err("offline")],
        };

        let error = cache
            .get_for_kid("https://issuer.example/expired-jwks", "known", &client)
            .await
            .expect_err("expired stale key must be rejected");
        assert!(matches!(error, ConnectError::Reqwest(_)));
    }

    #[test]
    fn rejects_a_stale_bound_shorter_than_ttl() {
        assert!(JwksCachePolicy::new(Duration::from_secs(2), Duration::from_secs(1)).is_err());
    }
}
