//! Bucket retention and peer-keying tests for [`RateLimiter`].

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use axum::http::Request;
use std::net::{IpAddr, SocketAddr};

fn peer_request(ip: &str) -> axum::extract::Request {
    let mut request = Request::builder().body(axum::body::Body::empty()).unwrap();
    let address = SocketAddr::new(ip.parse::<IpAddr>().unwrap(), 443);
    request
        .extensions_mut()
        .insert(axum::extract::ConnectInfo(address));
    request
}

#[test]
fn ipv6_peers_share_one_key_per_64_prefix() {
    let first = default_key_extractor(&peer_request("2001:db8:1:2::1"));
    let rotated = default_key_extractor(&peer_request("2001:db8:1:2:ffff:ee:dd:c"));
    let other_subnet = default_key_extractor(&peer_request("2001:db8:1:3::1"));

    assert_eq!(first, "2001:db8:1:2::/64");
    assert_eq!(rotated, first);
    assert_ne!(other_subnet, first);
    assert_eq!(
        default_key_extractor(&peer_request("::ffff:192.0.2.7")),
        "192.0.2.7"
    );
    assert_eq!(
        default_key_extractor(&peer_request("192.0.2.7")),
        "192.0.2.7"
    );
}

#[test]
fn rotating_ipv6_source_addresses_cannot_bypass_the_limit() {
    let limiter = RateLimiter::new(RateLimitConfig::per_hour(2.0));
    let decisions: Vec<bool> = (1..=4)
        .map(|host| {
            let key = default_key_extractor(&peer_request(&format!("2001:db8:7:9::{host:x}")));
            limiter.check_and_consume(&key)
        })
        .collect();

    assert_eq!(decisions, [true, true, false, false]);
}

#[test]
fn distinct_keys_cannot_grow_the_bucket_map_without_bound() {
    let limiter = RateLimiter::new(RateLimitConfig::per_hour(5.0));

    for client in 0..150_000 {
        limiter.check_and_consume(&format!("client-{client}"));
    }

    assert!(
        limiter.buckets.len() <= 100_000,
        "{} buckets retained",
        limiter.buckets.len()
    );
}

#[test]
fn refilled_buckets_are_released() {
    let limiter = RateLimiter::new(RateLimitConfig::per_second(1_000.0));
    for client in 0..5_000 {
        assert!(limiter.check_and_consume(&format!("burst-{client}")));
    }
    // One consumed token refills in a millisecond, after which the bucket is
    // identical to a new one.
    std::thread::sleep(Duration::from_millis(20));
    for client in 0..5_000 {
        assert!(limiter.check_and_consume(&format!("later-{client}")));
    }

    assert!(
        limiter.buckets.len() < 5_000,
        "{} buckets retained",
        limiter.buckets.len()
    );
}

#[test]
fn a_drained_bucket_is_not_reset_by_idle_sweeps() {
    let limiter = RateLimiter::new(RateLimitConfig::per_hour(2.0));
    assert!(limiter.check_and_consume("victim"));
    assert!(limiter.check_and_consume("victim"));

    for client in 0..10_000 {
        limiter.check_and_consume(&format!("client-{client}"));
    }

    assert!(!limiter.check_and_consume("victim"));
}

#[test]
fn a_full_map_evicts_the_least_recently_used_buckets() {
    let limiter = RateLimiter::new(RateLimitConfig::per_hour(1.0)).with_bucket_limit(16);
    for client in 0..16 {
        assert!(limiter.check_and_consume(&format!("client-{client}")));
    }
    std::thread::sleep(Duration::from_millis(2));
    assert!(!limiter.check_and_consume("client-15"));

    assert!(limiter.check_and_consume("newcomer"));

    assert!(limiter.buckets.len() <= 16);
    assert!(!limiter.buckets.contains_key("client-0"));
    assert!(!limiter.check_and_consume("client-15"));
    assert!(!limiter.check_and_consume("newcomer"));
}
