//! JWKS cache freshness, rotation and stale-fallback contracts.

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
            forced_refresh_at: None,
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

#[test]
fn policies_report_their_freshness_and_stale_bounds() {
    let default = JwksCachePolicy::default();
    assert_eq!(default.ttl(), Duration::from_secs(900));
    assert_eq!(default.max_stale(), Duration::from_secs(86_400));
    let custom = JwksCachePolicy::new(Duration::from_secs(60), Duration::from_secs(120))
        .expect("valid policy");
    assert_eq!(custom.ttl(), Duration::from_secs(60));
    assert_eq!(custom.max_stale(), Duration::from_secs(120));
}

#[tokio::test]
async fn get_serves_a_stale_set_on_refresh_error_only_within_the_stale_bound() {
    let policy =
        JwksCachePolicy::new(Duration::ZERO, Duration::from_secs(60)).expect("valid policy");
    let cache = JwksCache::new(policy);
    let client = SequenceClient {
        calls: AtomicUsize::new(0),
        responses: vec![Ok(jwks("known")), Err("offline")],
    };
    let url = "https://issuer.example/get-stale-jwks";
    cache.get(url, &client).await.expect("initial set");
    let stale = cache.get(url, &client).await.expect("bounded stale set");
    assert!(stale.find("known").is_some());
    assert_eq!(client.calls.load(Ordering::SeqCst), 2);

    let policy =
        JwksCachePolicy::new(Duration::ZERO, Duration::from_secs(1)).expect("valid policy");
    let expired = JwksCache::new(policy);
    let keys: JwkSet = serde_json::from_value(jwks("known")).expect("valid JWKS");
    expired.entries.write().await.insert(
        url.to_string(),
        CacheEntry {
            keys: Arc::new(keys),
            fetched_at: Instant::now()
                .checked_sub(Duration::from_secs(2))
                .expect("test instant supports a two-second offset"),
            forced_refresh_at: None,
        },
    );
    let offline = SequenceClient {
        calls: AtomicUsize::new(0),
        responses: vec![Err("offline")],
    };
    let error = expired
        .get(url, &offline)
        .await
        .expect_err("a set older than the stale bound must not be served");
    assert!(matches!(error, ConnectError::Reqwest(_)));
}

#[tokio::test]
async fn clear_forgets_fresh_sets() {
    let policy = JwksCachePolicy::new(Duration::from_secs(600), Duration::from_secs(1200))
        .expect("valid policy");
    let cache = JwksCache::new(policy);
    let client = SequenceClient {
        calls: AtomicUsize::new(0),
        responses: vec![Ok(jwks("first")), Ok(jwks("second"))],
    };
    let url = "https://issuer.example/cleared-jwks";
    assert!(
        cache
            .get(url, &client)
            .await
            .unwrap()
            .find("first")
            .is_some()
    );
    assert!(
        cache
            .get(url, &client)
            .await
            .unwrap()
            .find("first")
            .is_some()
    );
    assert_eq!(client.calls.load(Ordering::SeqCst), 1, "served fresh");

    cache.clear().await;
    assert!(
        cache
            .get(url, &client)
            .await
            .unwrap()
            .find("second")
            .is_some()
    );
    assert_eq!(client.calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn a_zero_forced_refresh_interval_never_throttles_an_unknown_kid() {
    let cache = JwksCache::new(JwksCachePolicy {
        ttl: Duration::from_secs(600),
        max_stale: Duration::from_secs(1200),
        min_forced_refresh_interval: Duration::ZERO,
    });
    let client = SequenceClient {
        calls: AtomicUsize::new(0),
        responses: vec![Ok(jwks("current")), Ok(jwks("rotated"))],
    };
    let url = "https://issuer.example/unthrottled-jwks";
    cache
        .get_for_kid(url, "current", &client)
        .await
        .expect("initial key");
    // A mark the clock has not reached yet: the time since it saturates to zero.
    cache
        .entries
        .write()
        .await
        .get_mut(url)
        .expect("cached entry")
        .forced_refresh_at = Some(
        Instant::now()
            .checked_add(Duration::from_secs(3_600))
            .expect("test instant supports a one-hour offset"),
    );
    let rotated = cache
        .get_for_kid(url, "rotated", &client)
        .await
        .expect("an unknown kid still refreshes");
    assert!(rotated.find("rotated").is_some());
    assert_eq!(client.calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn the_process_cache_returns_the_fetched_set_for_a_kid() {
    let client = SequenceClient {
        calls: AtomicUsize::new(0),
        responses: vec![Ok(jwks("process-kid"))],
    };
    // A URL no other test uses keeps the process-wide cache entry private.
    let keys = fetch_and_cache_jwks_for_kid(
        "https://process-cache.example/jwks-for-kid",
        "process-kid",
        &client,
    )
    .await
    .expect("fetched set");
    assert!(keys.find("process-kid").is_some());
}
