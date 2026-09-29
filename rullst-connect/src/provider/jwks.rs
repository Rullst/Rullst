//! Bounded JWKS caching with rotation-aware refresh and safe stale fallback.
//!
//! A token's `kid` is attacker-controlled until its signature is verified, so
//! an unknown `kid` may force at most one refresh of a fresh cached set per 30
//! seconds and URL. Refreshes of one URL are single-flight: concurrent callers
//! wait for, and reuse, the refresh in progress.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};

use jsonwebtoken::jwk::JwkSet;
use tokio::sync::{Mutex, RwLock};

use crate::client::{HttpClient, HttpClientExt};
use crate::error::ConnectError;

const DEFAULT_TTL: Duration = Duration::from_secs(15 * 60);
const DEFAULT_MAX_STALE: Duration = Duration::from_secs(24 * 60 * 60);
/// Minimum time between refreshes that an unknown `kid` may force while the
/// cached set is still fresh. The first unknown `kid` after the interval
/// refreshes again, so key rotation keeps working.
pub(crate) const MIN_FORCED_REFRESH_INTERVAL: Duration = Duration::from_secs(30);
/// Longest accepted `kid`, checked before any cache lookup or network I/O.
pub(crate) const MAX_KID_BYTES: usize = 256;

/// Freshness and stale-on-error bounds for a JWKS cache.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct JwksCachePolicy {
    ttl: Duration,
    max_stale: Duration,
    min_forced_refresh_interval: Duration,
}

impl JwksCachePolicy {
    /// Creates a policy. `max_stale` is the maximum total age of a cached set.
    ///
    /// While a set is younger than `ttl`, an unknown `kid` forces at most one
    /// refresh every 30 seconds; a zero `ttl` refreshes on every lookup.
    pub fn new(ttl: Duration, max_stale: Duration) -> Result<Self, ConnectError> {
        if max_stale < ttl {
            return Err(ConnectError::InvalidConfiguration {
                field: "jwks_max_stale",
                reason: "must be greater than or equal to the JWKS TTL".to_string(),
            });
        }
        Ok(Self {
            ttl,
            max_stale,
            min_forced_refresh_interval: MIN_FORCED_REFRESH_INTERVAL,
        })
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
            min_forced_refresh_interval: MIN_FORCED_REFRESH_INTERVAL,
        }
    }
}

#[derive(Clone)]
struct CacheEntry {
    keys: Arc<JwkSet>,
    fetched_at: Instant,
    /// Last refresh, successful or not, forced by an unknown `kid` while fresh.
    forced_refresh_at: Option<Instant>,
}

/// An isolated JWKS cache. Providers own a cache so injected clients cannot
/// accidentally share trust material with another provider instance.
#[derive(Clone)]
pub struct JwksCache {
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
    /// One refresh gate per URL makes remote fetches single-flight.
    refresh_gates: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
    policy: JwksCachePolicy,
}

/// Result of consulting a fresh cached set for a `kid`.
enum FreshLookup {
    Found(Arc<JwkSet>),
    Throttled,
    Refresh,
}

impl JwksCache {
    /// Creates an empty cache with the supplied freshness policy.
    pub fn new(policy: JwksCachePolicy) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            refresh_gates: Arc::new(Mutex::new(HashMap::new())),
            policy,
        }
    }

    /// Fetches a key set, refreshing entries after their TTL.
    pub async fn get(
        &self,
        url: &str,
        client: &dyn HttpClient,
    ) -> Result<Arc<JwkSet>, ConnectError> {
        if let Some(entry) = self.cached(url).await
            && self.is_fresh(&entry)
        {
            return Ok(entry.keys);
        }

        let gate = self.refresh_gate(url).await;
        let _single_flight = gate.lock().await;
        // Another caller may have refreshed the set while this one waited.
        let cached = self.cached(url).await;
        if let Some(entry) = cached.as_ref()
            && self.is_fresh(entry)
        {
            return Ok(entry.keys.clone());
        }

        match fetch_remote(url, client).await {
            Ok(keys) => {
                let forced_refresh_at = cached.and_then(|entry| entry.forced_refresh_at);
                self.store(url, keys.clone(), forced_refresh_at).await;
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

    /// Returns a set containing `kid`.
    ///
    /// A `kid` that is empty, longer than 256 bytes or not printable ASCII is
    /// rejected before any I/O. A missing key forces a refresh even while the
    /// cached set is fresh, which supports key rotation, but at most once per
    /// 30 seconds and URL: until then an unknown `kid` fails without a network
    /// call. Concurrent refreshes of one URL are coalesced.
    pub async fn get_for_kid(
        &self,
        url: &str,
        kid: &str,
        client: &dyn HttpClient,
    ) -> Result<Arc<JwkSet>, ConnectError> {
        validate_kid(kid)?;

        match self.lookup_fresh(url, kid).await {
            FreshLookup::Found(keys) => return Ok(keys),
            FreshLookup::Throttled => return Err(ConnectError::JwkNotFound(kid.to_string())),
            FreshLookup::Refresh => {}
        }

        let gate = self.refresh_gate(url).await;
        let _single_flight = gate.lock().await;
        // A concurrent caller may have refreshed, or been throttled, meanwhile.
        match self.lookup_fresh(url, kid).await {
            FreshLookup::Found(keys) => return Ok(keys),
            FreshLookup::Throttled => return Err(ConnectError::JwkNotFound(kid.to_string())),
            FreshLookup::Refresh => {}
        }

        let cached = self.cached(url).await;
        let forced = cached.as_ref().is_some_and(|entry| self.is_fresh(entry));
        let attempted_at = Instant::now();
        match fetch_remote(url, client).await {
            Ok(keys) => {
                let forced_refresh_at = if forced {
                    Some(attempted_at)
                } else {
                    cached.as_ref().and_then(|entry| entry.forced_refresh_at)
                };
                self.store(url, keys.clone(), forced_refresh_at).await;
                if keys.find(kid).is_some() {
                    Ok(keys)
                } else {
                    Err(ConnectError::JwkNotFound(kid.to_string()))
                }
            }
            Err(error) => {
                if forced {
                    self.record_forced_refresh(url, attempted_at).await;
                }
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

    fn is_fresh(&self, entry: &CacheEntry) -> bool {
        age(entry) <= self.policy.ttl
    }

    async fn lookup_fresh(&self, url: &str, kid: &str) -> FreshLookup {
        let Some(entry) = self.cached(url).await else {
            return FreshLookup::Refresh;
        };
        if !self.is_fresh(&entry) {
            return FreshLookup::Refresh;
        }
        if entry.keys.find(kid).is_some() {
            return FreshLookup::Found(entry.keys);
        }
        let recently_forced = entry.forced_refresh_at.is_some_and(|at| {
            Instant::now().saturating_duration_since(at) < self.policy.min_forced_refresh_interval
        });
        if recently_forced {
            FreshLookup::Throttled
        } else {
            FreshLookup::Refresh
        }
    }

    async fn refresh_gate(&self, url: &str) -> Arc<Mutex<()>> {
        self.refresh_gates
            .lock()
            .await
            .entry(url.to_string())
            .or_default()
            .clone()
    }

    async fn store(&self, url: &str, keys: Arc<JwkSet>, forced_refresh_at: Option<Instant>) {
        self.entries.write().await.insert(
            url.to_string(),
            CacheEntry {
                keys,
                fetched_at: Instant::now(),
                forced_refresh_at,
            },
        );
    }

    /// Throttles a failed forced refresh like a successful one.
    async fn record_forced_refresh(&self, url: &str, attempted_at: Instant) {
        if let Some(entry) = self.entries.write().await.get_mut(url) {
            entry.forced_refresh_at = Some(attempted_at);
        }
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

/// Rejects a `kid` that no provider key set could legitimately use, without
/// echoing the untrusted value.
fn validate_kid(kid: &str) -> Result<(), ConnectError> {
    if kid.is_empty() || kid.len() > MAX_KID_BYTES || !kid.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(ConnectError::JwkNotFound("<invalid>".to_string()));
    }
    Ok(())
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

/// Fetches JWKS and forces a rate-limited refresh when `kid` is absent from a
/// fresh cache. See [`JwksCache::get_for_kid`].
pub async fn fetch_and_cache_jwks_for_kid(
    url: &str,
    kid: &str,
    client: &dyn HttpClient,
) -> Result<Arc<JwkSet>, ConnectError> {
    PROCESS_JWKS_CACHE.get_for_kid(url, kid, client).await
}

#[cfg(test)]
mod refresh_tests;

#[cfg(test)]
mod tests;
