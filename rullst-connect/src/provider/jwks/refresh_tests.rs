//! Unknown-`kid` refresh throttling against a counting loopback JWKS server.

use super::*;
use crate::client::ReqwestClient;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn key_set(kids: &[&str]) -> serde_json::Value {
    let keys: Vec<_> = kids
        .iter()
        .map(|kid| {
            json!({
                "kty": "RSA", "kid": kid, "use": "sig", "alg": "RS256",
                "n": "sXchDaQebHnPiGvyDO5R", "e": "AQAB"
            })
        })
        .collect();
    json!({ "keys": keys })
}

async fn serve(server: &MockServer, kids: &[&str]) {
    server.reset().await;
    Mock::given(method("GET"))
        .and(path("/jwks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(key_set(kids)))
        .mount(server)
        .await;
}

async fn fetches(server: &MockServer) -> usize {
    server
        .received_requests()
        .await
        .map_or(0, |requests| requests.len())
}

fn cache() -> JwksCache {
    let policy = JwksCachePolicy::new(Duration::from_secs(600), Duration::from_secs(1200))
        .expect("valid policy");
    JwksCache::new(policy)
}

#[tokio::test]
#[cfg(not(miri))]
async fn repeated_unknown_kids_cause_at_most_one_forced_fetch_per_interval() {
    let server = MockServer::start().await;
    serve(&server, &["current"]).await;
    let url = format!("{}/jwks", server.uri());
    let client = ReqwestClient::new();
    let cache = cache();

    cache
        .get_for_kid(&url, "current", &client)
        .await
        .expect("initial key");
    assert_eq!(fetches(&server).await, 1);

    for attempt in 0..25 {
        let error = cache
            .get_for_kid(&url, &format!("attacker-{attempt}"), &client)
            .await
            .expect_err("unknown kid must be rejected");
        assert!(matches!(error, ConnectError::JwkNotFound(_)), "{error:?}");
    }
    assert_eq!(
        fetches(&server).await,
        2,
        "only the first unknown kid may force a refresh within the interval"
    );

    cache
        .get_for_kid(&url, "current", &client)
        .await
        .expect("known key is still served from the fresh cache");
    assert_eq!(fetches(&server).await, 2);
}

#[tokio::test]
#[cfg(not(miri))]
async fn concurrent_unknown_kids_share_one_forced_fetch() {
    let server = MockServer::start().await;
    serve(&server, &["current"]).await;
    let url = format!("{}/jwks", server.uri());
    let client = Arc::new(ReqwestClient::new());
    let cache = cache();
    cache
        .get_for_kid(&url, "current", client.as_ref())
        .await
        .expect("initial key");

    let tasks: Vec<_> = (0..16)
        .map(|index| {
            let (cache, client, url) = (cache.clone(), client.clone(), url.clone());
            tokio::spawn(async move {
                cache
                    .get_for_kid(&url, &format!("concurrent-{index}"), client.as_ref())
                    .await
            })
        })
        .collect();
    for task in tasks {
        assert!(task.await.expect("task completes").is_err());
    }
    assert_eq!(fetches(&server).await, 2, "one coalesced forced refresh");
}

#[tokio::test]
#[cfg(not(miri))]
async fn malformed_kids_fail_before_any_network_io() {
    let server = MockServer::start().await;
    serve(&server, &["current"]).await;
    let url = format!("{}/jwks", server.uri());
    let client = ReqwestClient::new();
    let cache = cache();

    for kid in [
        String::new(),
        " ".to_string(),
        "k".repeat(MAX_KID_BYTES + 1),
        "with space".to_string(),
        "line\nbreak".to_string(),
        "nul\0byte".to_string(),
        "chave-çã".to_string(),
    ] {
        let error = cache
            .get_for_kid(&url, &kid, &client)
            .await
            .expect_err("malformed kid must be rejected");
        let ConnectError::JwkNotFound(reported) = error else {
            panic!("unexpected error {error:?}");
        };
        assert!(
            reported.len() <= 16,
            "attacker kid is not echoed: {reported}"
        );
    }
    assert_eq!(fetches(&server).await, 0);

    let longest = "k".repeat(MAX_KID_BYTES);
    assert!(cache.get_for_kid(&url, &longest, &client).await.is_err());
    assert_eq!(fetches(&server).await, 1, "a bounded kid may still fetch");
}

/// Moves the last forced refresh of `url` back past the minimum interval.
async fn elapse_forced_refresh_interval(cache: &JwksCache, url: &str) {
    let mut entries = cache.entries.write().await;
    let entry = entries.get_mut(url).expect("cached entry");
    entry.forced_refresh_at = Some(
        Instant::now()
            .checked_sub(MIN_FORCED_REFRESH_INTERVAL + Duration::from_secs(1))
            .expect("test instant supports the interval offset"),
    );
}

#[tokio::test]
#[cfg(not(miri))]
async fn rotated_keys_are_picked_up_by_the_first_unknown_kid_after_the_interval() {
    let server = MockServer::start().await;
    serve(&server, &["first"]).await;
    let url = format!("{}/jwks", server.uri());
    let client = ReqwestClient::new();
    let cache = cache();
    cache
        .get_for_kid(&url, "first", &client)
        .await
        .expect("initial key");

    // The first unknown kid refreshes immediately and finds the rotated key.
    serve(&server, &["first", "second"]).await;
    let rotated = cache
        .get_for_kid(&url, "second", &client)
        .await
        .expect("first rotation is picked up at once");
    assert!(rotated.find("second").is_some());
    assert_eq!(
        fetches(&server).await,
        1,
        "counter resets with the new mock"
    );

    // A second rotation inside the interval is not fetched yet.
    serve(&server, &["second", "third"]).await;
    assert!(cache.get_for_kid(&url, "third", &client).await.is_err());
    assert_eq!(fetches(&server).await, 0);

    elapse_forced_refresh_interval(&cache, &url).await;
    let rotated = cache
        .get_for_kid(&url, "third", &client)
        .await
        .expect("first unknown kid after the interval refreshes");
    assert!(rotated.find("third").is_some());
    assert_eq!(fetches(&server).await, 1);
}

#[tokio::test]
#[cfg(not(miri))]
async fn failed_forced_refreshes_are_throttled_and_keep_known_keys() {
    let server = MockServer::start().await;
    serve(&server, &["current"]).await;
    let url = format!("{}/jwks", server.uri());
    let client = ReqwestClient::new();
    let cache = cache();
    cache
        .get_for_kid(&url, "current", &client)
        .await
        .expect("initial key");

    // A non-transient status keeps the optional retry middleware to one request.
    server.reset().await;
    Mock::given(method("GET"))
        .and(path("/jwks"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    let error = cache
        .get_for_kid(&url, "unknown-1", &client)
        .await
        .expect_err("failed refresh cannot supply an unknown key");
    assert!(matches!(error, ConnectError::ProviderApiError { .. }));
    for attempt in 2..10 {
        assert!(
            cache
                .get_for_kid(&url, &format!("unknown-{attempt}"), &client)
                .await
                .is_err()
        );
    }
    assert_eq!(
        fetches(&server).await,
        1,
        "a failed forced refresh throttles"
    );

    cache
        .get_for_kid(&url, "current", &client)
        .await
        .expect("known key remains available from the fresh set");
    assert_eq!(fetches(&server).await, 1);
}
