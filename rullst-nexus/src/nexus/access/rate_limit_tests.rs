//! Failed-credential guard of the Nexus Basic Auth policy.

use super::*;

fn limiter(max_buckets: usize) -> BasicAuthRateLimiter {
    BasicAuthRateLimiter {
        max_buckets,
        ..BasicAuthRateLimiter::default()
    }
}

fn ip(value: &str) -> IpAddr {
    value.parse().expect("valid test address")
}

fn bucket_count(limiter: &BasicAuthRateLimiter) -> usize {
    limiter
        .state
        .lock()
        .expect("unpoisoned guard")
        .buckets
        .len()
}

#[test]
fn ipv6_peers_share_their_64_prefix_and_mapped_ipv4_is_ipv4() {
    assert_eq!(
        client_bucket(ip("2001:db8:1:2:aaaa::1")),
        client_bucket(ip("2001:db8:1:2:ffff:ffff:ffff:ffff"))
    );
    assert_ne!(
        client_bucket(ip("2001:db8:1:2::1")),
        client_bucket(ip("2001:db8:1:3::1"))
    );
    assert_eq!(client_bucket(ip("::ffff:192.0.2.7")), ip("192.0.2.7"));
    assert_ne!(
        client_bucket(ip("192.0.2.7")),
        client_bucket(ip("192.0.2.8"))
    );

    let limiter = limiter(NEXUS_BASIC_AUTH_MAX_PEERS);
    for index in 1..=NEXUS_BASIC_AUTH_MAX_FAILURES {
        limiter.record_failure(ip(&format!("2001:db8:1:2::{index:x}")));
    }
    assert!(matches!(
        limiter.status(ip("2001:db8:1:2::ffff")),
        AuthGuardStatus::Locked(_)
    ));
    assert!(matches!(
        limiter.status(ip("2001:db8:1:3::1")),
        AuthGuardStatus::Allowed
    ));
}

#[test]
fn expired_buckets_are_swept_on_an_interval_not_per_request() {
    let limiter = limiter(NEXUS_BASIC_AUTH_MAX_PEERS);
    let start = Instant::now();
    limiter.status_at(ip("192.0.2.1"), start);
    for index in 1..=10 {
        limiter.record_failure_at(ip(&format!("192.0.2.{index}")), start);
    }
    assert_eq!(bucket_count(&limiter), 10);

    let expired = start + NEXUS_BASIC_AUTH_FAILURE_WINDOW + Duration::from_secs(1);
    // Every stored bucket is expired, but the last sweep ran at `start`
    // and the next one is not due, so this lookup must not scan the map.
    let not_due = start + PRUNE_INTERVAL - Duration::from_secs(1);
    limiter.status_at(ip("198.51.100.1"), not_due);
    assert_eq!(bucket_count(&limiter), 10);

    limiter.status_at(ip("198.51.100.1"), expired);
    assert_eq!(bucket_count(&limiter), 0);
}

#[test]
fn capacity_eviction_keeps_the_map_bounded() {
    let limiter = limiter(4);
    let now = Instant::now();
    for index in 1..=20 {
        limiter.record_failure_at(ip(&format!("203.0.113.{index}")), now);
        assert!(bucket_count(&limiter) <= 4);
    }
}

#[test]
fn documented_limits_and_debug_output_are_exact() {
    assert_eq!(NEXUS_BASIC_AUTH_MAX_FAILURES, 5);
    assert_eq!(NEXUS_BASIC_AUTH_FAILURE_WINDOW, Duration::from_secs(300));
    assert_eq!(NEXUS_BASIC_AUTH_LOCKOUT, Duration::from_secs(900));
    assert_eq!(NEXUS_BASIC_AUTH_MAX_PEERS, 100_000);

    let limiter = limiter(NEXUS_BASIC_AUTH_MAX_PEERS);
    let cookie = limiter.known_client_cookie().expect("valid cookie header");
    assert_eq!(
        cookie.to_str().expect("ASCII cookie"),
        format!(
            "rullst_nexus_known_client={}; Path=/; Max-Age=2592000; HttpOnly; Secure; SameSite=Lax",
            limiter.known_client_token
        )
    );
    assert_eq!(
        format!("{limiter:?}"),
        "BasicAuthRateLimiter { known_client_token: \"[REDACTED]\", max_failures: 5, .. }"
    );
    assert_eq!(
        client_bucket(ip("2001:db8:1:2:aaaa:bbbb:cccc:dddd")),
        ip("2001:db8:1:2::")
    );
}

#[test]
fn the_fifth_failure_locks_until_exactly_the_lockout_ends() {
    let limiter = limiter(NEXUS_BASIC_AUTH_MAX_PEERS);
    let start = Instant::now();
    let peer = ip("192.0.2.10");
    for _ in 1..NEXUS_BASIC_AUTH_MAX_FAILURES {
        assert!(matches!(
            limiter.record_failure_at(peer, start),
            AuthGuardStatus::Allowed
        ));
        assert!(matches!(
            limiter.status_at(peer, start),
            AuthGuardStatus::Allowed
        ));
    }
    assert!(matches!(
        limiter.record_failure_at(peer, start),
        AuthGuardStatus::Locked(lockout) if lockout == Duration::from_secs(900)
    ));
    let almost = start + Duration::from_secs(899);
    assert!(matches!(
        limiter.status_at(peer, almost),
        AuthGuardStatus::Locked(remaining) if remaining == Duration::from_secs(1)
    ));
    let ended = start + Duration::from_secs(900);
    assert!(matches!(
        limiter.status_at(peer, ended),
        AuthGuardStatus::Allowed
    ));
    assert_eq!(bucket_count(&limiter), 0, "an ended lockout is forgotten");
}

#[test]
fn failures_count_within_a_window_measured_from_its_first_failure() {
    let window = Duration::from_secs(300);
    let start = Instant::now();
    let recent = start + window - Duration::from_secs(10);

    // The fifth failure exactly one window after the first still counts.
    let limiter = limiter(NEXUS_BASIC_AUTH_MAX_PEERS);
    let peer = ip("192.0.2.20");
    limiter.record_failure_at(peer, start);
    for _ in 0..3 {
        limiter.record_failure_at(peer, recent);
    }
    assert!(matches!(
        limiter.record_failure_at(peer, start + window),
        AuthGuardStatus::Locked(_)
    ));

    // One second later a new window starts with this failure.
    let limiter = self::limiter(NEXUS_BASIC_AUTH_MAX_PEERS);
    limiter.record_failure_at(peer, start);
    for _ in 0..3 {
        limiter.record_failure_at(peer, recent);
    }
    assert!(matches!(
        limiter.record_failure_at(peer, start + window + Duration::from_secs(1)),
        AuthGuardStatus::Allowed
    ));
    for _ in 0..3 {
        limiter.record_failure_at(peer, start + window + Duration::from_secs(1));
    }
    assert!(matches!(
        limiter.record_failure_at(peer, start + window + Duration::from_secs(1)),
        AuthGuardStatus::Locked(_)
    ));
}

#[test]
fn sweeps_run_exactly_when_due_and_keep_active_lockouts() {
    let limiter = limiter(NEXUS_BASIC_AUTH_MAX_PEERS);
    let start = Instant::now();
    let stale = ip("192.0.2.30");
    let probe = ip("198.51.100.30");
    limiter.record_failure_at(stale, start);
    // A due sweep 280 s later keeps the bucket and schedules the next one
    // for 310 s, when the bucket is 10 s past its window.
    limiter.status_at(probe, start + Duration::from_secs(280));
    assert_eq!(bucket_count(&limiter), 1);
    limiter.status_at(probe, start + Duration::from_secs(310));
    assert_eq!(bucket_count(&limiter), 0, "the due sweep did not run");

    // A locked bucket outlives its failure window until the lockout ends.
    let limiter = self::limiter(NEXUS_BASIC_AUTH_MAX_PEERS);
    let locked = ip("192.0.2.31");
    for _ in 0..NEXUS_BASIC_AUTH_MAX_FAILURES {
        limiter.record_failure_at(locked, start);
    }
    limiter.status_at(probe, start + Duration::from_secs(400));
    assert_eq!(bucket_count(&limiter), 1, "an active lockout was swept");
    assert!(matches!(
        limiter.status_at(locked, start + Duration::from_secs(400)),
        AuthGuardStatus::Locked(remaining) if remaining == Duration::from_secs(500)
    ));
    // `status_at` refreshed the bucket at 400 s; at 900 s the lockout has
    // ended and the bucket is 500 s past that refresh.
    limiter.status_at(probe, start + Duration::from_secs(900));
    assert_eq!(bucket_count(&limiter), 0, "an ended lockout was kept");
}

#[test]
fn eviction_happens_only_at_capacity_and_removes_the_stalest_bucket() {
    let limiter = limiter(3);
    let start = Instant::now();
    for (offset, peer) in ["203.0.113.1", "203.0.113.2", "203.0.113.3"]
        .into_iter()
        .enumerate()
    {
        limiter.record_failure_at(ip(peer), start + Duration::from_secs(offset as u64));
    }
    assert_eq!(
        bucket_count(&limiter),
        3,
        "a bucket was evicted below capacity"
    );
    // A known bucket at capacity is updated, not evicted.
    limiter.record_failure_at(ip("203.0.113.1"), start + Duration::from_secs(3));
    assert_eq!(bucket_count(&limiter), 3);

    limiter.record_failure_at(ip("203.0.113.4"), start + Duration::from_secs(4));
    let buckets = limiter
        .state
        .lock()
        .expect("unpoisoned guard")
        .buckets
        .keys()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        buckets,
        ["203.0.113.1", "203.0.113.3", "203.0.113.4"]
            .into_iter()
            .map(ip)
            .collect()
    );
}

#[test]
fn known_client_cookie_requires_the_exact_process_token() {
    let limiter = limiter(NEXUS_BASIC_AUTH_MAX_PEERS);
    let cookie = limiter.known_client_cookie().expect("valid cookie header");
    let cookie = cookie.to_str().expect("ASCII cookie");
    assert!(cookie.contains("HttpOnly") && cookie.contains("Secure"));
    let pair = cookie.split(';').next().expect("cookie pair");

    let mut headers = HeaderMap::new();
    assert!(!limiter.is_known_client(&headers));
    headers.insert(
        header::COOKIE,
        HeaderValue::from_str(&format!("rullst_csrf=abc; {pair}")).expect("cookie"),
    );
    assert!(limiter.is_known_client(&headers));

    let other = BasicAuthRateLimiter::default();
    assert!(!other.is_known_client(&headers));
    headers.insert(
        header::COOKIE,
        HeaderValue::from_static("rullst_nexus_known_client=forged"),
    );
    assert!(!limiter.is_known_client(&headers));

    // The right value under another name, and a same-length wrong value.
    let token = limiter.known_client_token.clone();
    let same_length = "x".repeat(token.len());
    for cookie in [
        format!("other_cookie={token}"),
        format!("rullst_nexus_known_client={same_length}"),
    ] {
        headers.insert(
            header::COOKIE,
            HeaderValue::from_str(&cookie).expect("cookie"),
        );
        assert!(!limiter.is_known_client(&headers), "{cookie}");
    }
}
