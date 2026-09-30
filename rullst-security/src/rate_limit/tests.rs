use super::*;
use axum::{Router, body::Body, extract::ConnectInfo, http::Request, middleware, routing::get};
use tower::ServiceExt;

#[test]
fn zero_budget_denies_requests_even_after_a_window_reset() {
    let limiter = RateLimiter::new(0, Duration::from_millis(1));
    limiter.store.insert(
        "no-budget".to_string(),
        (Instant::now() - Duration::from_secs(1), AtomicU64::new(99)),
    );
    assert!(limiter.check("no-budget"));
    assert!(RateLimiter::new(1, Duration::ZERO).check("zero-window"));
}

#[test]
fn arbitrary_client_keys_cannot_grow_memory_without_a_bound() {
    let limiter = RateLimiter::new(1, Duration::from_secs(60));
    assert!(limiter.check(&"x".repeat(257)));
    assert!(limiter.check(""));
    for index in 0..16_384 {
        assert!(!limiter.check(&format!("bounded-client-{index}")));
    }
    assert!(limiter.check("over-capacity-client"));
    assert_eq!(limiter.store.len(), 16_384);
}

#[test]
fn counter_saturation_cannot_reopen_the_request_budget() {
    let limiter = RateLimiter::new(u64::MAX, Duration::from_secs(60));
    limiter.store.insert(
        "saturated".to_string(),
        (Instant::now(), AtomicU64::new(u64::MAX)),
    );
    assert!(limiter.check("saturated"));
}

#[test]
fn expired_identities_are_reclaimed_without_a_background_runtime() {
    let limiter = RateLimiter::new(1, Duration::from_secs(60));
    let expired = Instant::now() - Duration::from_secs(120);
    for index in 0..MAX_RATE_LIMIT_IDENTITIES {
        limiter
            .store
            .insert(format!("expired-{index}"), (expired, AtomicU64::new(1)));
    }
    limiter.admission.lock().unwrap().last_cleanup = expired;
    assert!(!limiter.check("new-identity"));
    assert_eq!(limiter.store.len(), 1);
}

#[test]
fn concurrent_new_identities_share_one_remaining_slot() {
    let limiter = RateLimiter::new(1, Duration::from_secs(60));
    for index in 1..MAX_RATE_LIMIT_IDENTITIES {
        limiter.store.insert(
            format!("occupied-{index}"),
            (Instant::now(), AtomicU64::new(1)),
        );
    }
    let barrier = std::sync::Barrier::new(16);
    let admitted = AtomicU64::new(0);
    std::thread::scope(|scope| {
        for index in 0..16 {
            let (limiter, barrier, admitted) = (&limiter, &barrier, &admitted);
            scope.spawn(move || {
                barrier.wait();
                if !limiter.check(&format!("contender-{index}")) {
                    admitted.fetch_add(1, Ordering::Relaxed);
                }
            });
        }
    });
    assert_eq!(admitted.load(Ordering::Relaxed), 1);
    assert_eq!(limiter.store.len(), MAX_RATE_LIMIT_IDENTITIES);
}

#[test]
fn test_sliding_window_rate_limiter() {
    let ip = "192.168.99.1";
    let window = Duration::from_secs(10);
    assert!(!is_rate_limited(ip, 3, window));
    assert!(!is_rate_limited(ip, 3, window));
    assert!(!is_rate_limited(ip, 3, window));
    // 4th request exceeds max_requests=3
    assert!(is_rate_limited(ip, 3, window));
}

#[test]
fn global_helper_keeps_a_separate_budget_per_policy_on_the_same_key() {
    let key = "per-policy-regression-key";
    let hourly = Duration::from_secs(3_600);
    let short = Duration::from_millis(1);

    assert!(!is_rate_limited(key, 2, hourly));
    assert!(!is_rate_limited(key, 2, hourly));
    assert!(is_rate_limited(key, 2, hourly));

    // A shorter window on the same key has its own budget and resetting it
    // must not reopen the hourly budget.
    std::thread::sleep(Duration::from_millis(5));
    assert!(!is_rate_limited(key, 5, short));
    std::thread::sleep(Duration::from_millis(5));
    assert!(!is_rate_limited(key, 5, short));
    assert!(is_rate_limited(key, 2, hourly));

    // Same window, different maximum: counts are not shared either.
    for _ in 0..3 {
        assert!(!is_rate_limited(key, 3, hourly));
    }
    assert!(is_rate_limited(key, 3, hourly));
    assert!(is_rate_limited(key, 2, hourly));
}

#[test]
fn test_rate_limiter_builder() {
    let limiter = RateLimiter::new(5, Duration::from_secs(1));
    assert_eq!(limiter.max_requests, 5);
    let key = "10.0.0.1";
    for _ in 0..5 {
        assert!(!limiter.check(key));
    }
    assert!(limiter.check(key));
    assert!(matches!(
        RateLimiter::new(5, Duration::from_secs(1)).try_with_distributed(),
        Err(RateLimitError::DistributedBackendUnsupported)
    ));
}

#[test]
#[allow(deprecated)]
fn distributed_compatibility_builder_fails_closed() {
    let limiter = RateLimiter::new(10, Duration::from_secs(1)).with_distributed();
    assert_eq!(limiter.backend, RateLimitBackend::Distributed);
    assert!(limiter.check("distributed-client"));
}

#[test]
fn expired_window_resets_request_count() {
    let limiter = RateLimiter::new(1, Duration::from_millis(1));
    limiter.store.insert(
        "expired-client".to_string(),
        (Instant::now() - Duration::from_secs(1), AtomicU64::new(99)),
    );
    assert!(!limiter.check("expired-client"));
    assert!(limiter.check("expired-client"));
}

#[tokio::test]
async fn middleware_fails_closed_without_peer_address() {
    let app = Router::new()
        .route("/", get(|| async { StatusCode::OK }))
        .layer(middleware::from_fn(rate_limit_middleware));
    let response = app
        .oneshot(
            Request::get("/")
                .body(Body::empty())
                .expect("request should be valid"),
        )
        .await
        .expect("middleware request should complete");
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn middleware_returns_too_many_requests_after_default_limit() {
    let app = Router::new()
        .route("/", get(|| async { StatusCode::OK }))
        .layer(middleware::from_fn(rate_limit_middleware));
    let peer = ConnectInfo(
        "192.0.2.217:4123"
            .parse::<std::net::SocketAddr>()
            .expect("socket address should be valid"),
    );

    for expected in std::iter::repeat_n(StatusCode::OK, 120)
        .chain(std::iter::once(StatusCode::TOO_MANY_REQUESTS))
    {
        let mut request = Request::get("/")
            .body(Body::empty())
            .expect("request should be valid");
        request.extensions_mut().insert(peer);
        let response = app
            .clone()
            .oneshot(request)
            .await
            .expect("middleware request should complete");
        assert_eq!(response.status(), expected);
    }
}

fn peer_request(peer: &str) -> Request<Body> {
    let mut request = Request::get("/")
        .body(Body::empty())
        .expect("request should be valid");
    request.extensions_mut().insert(ConnectInfo(
        peer.parse::<std::net::SocketAddr>()
            .expect("socket address should be valid"),
    ));
    request
}

#[tokio::test]
async fn middleware_shares_one_budget_across_an_ipv6_64_prefix() {
    let app = Router::new()
        .route("/", get(|| async { StatusCode::OK }))
        .layer(middleware::from_fn(rate_limit_middleware));

    // Every request uses a different interface ID inside one /64.
    for index in 0..120_u32 {
        let peer = format!("[2001:db8:5:7:{index:x}::1]:443");
        let response = app.clone().oneshot(peer_request(&peer)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
    let rotated = app
        .clone()
        .oneshot(peer_request("[2001:db8:5:7:ffff:1:2:3]:443"))
        .await
        .unwrap();
    assert_eq!(rotated.status(), StatusCode::TOO_MANY_REQUESTS);

    // A neighbouring /64 keeps its own budget.
    let neighbour = app
        .oneshot(peer_request("[2001:db8:5:8::1]:443"))
        .await
        .unwrap();
    assert_eq!(neighbour.status(), StatusCode::OK);
}

#[tokio::test]
async fn middleware_keys_ipv4_mapped_ipv6_peers_as_ipv4() {
    let app = Router::new()
        .route("/", get(|| async { StatusCode::OK }))
        .layer(middleware::from_fn(rate_limit_middleware));

    for _ in 0..120 {
        let response = app
            .clone()
            .oneshot(peer_request("192.0.2.218:4000"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
    let mapped = app
        .oneshot(peer_request("[::ffff:192.0.2.218]:4001"))
        .await
        .unwrap();
    assert_eq!(mapped.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[test]
fn peer_keys_group_ipv6_by_64_and_keep_ipv4_per_address() {
    let key = |ip: &str| peer_rate_limit_key(ip.parse().unwrap());
    assert_eq!(key("192.0.2.1"), "192.0.2.1");
    assert_eq!(key("::ffff:192.0.2.1"), "192.0.2.1");
    assert_eq!(key("2001:db8:0:1::1"), "2001:db8:0:1::/64");
    assert_eq!(
        key("2001:db8:0:1::1"),
        key("2001:db8:0:1:ffff:ffff:ffff:ffff")
    );
    assert_ne!(key("2001:db8:0:1::1"), key("2001:db8:0:2::1"));
}
