use axum::http::{HeaderMap, HeaderValue, header};
use std::{
    collections::HashMap,
    fmt,
    net::{IpAddr, Ipv6Addr},
    sync::Mutex,
    time::{Duration, Instant},
};
use subtle::ConstantTimeEq;

/// Number of failed Basic credentials allowed per client bucket (one IPv4
/// address or one IPv6 /64) before the default lockout starts.
pub const NEXUS_BASIC_AUTH_MAX_FAILURES: u32 = 5;
/// Window in which Basic Auth failures are accumulated.
pub const NEXUS_BASIC_AUTH_FAILURE_WINDOW: Duration = Duration::from_secs(5 * 60);
/// Default lockout duration after too many Basic Auth failures.
pub const NEXUS_BASIC_AUTH_LOCKOUT: Duration = Duration::from_secs(15 * 60);
/// Maximum number of client buckets retained by the Basic Auth guard.
pub const NEXUS_BASIC_AUTH_MAX_PEERS: usize = 100_000;

/// Minimum interval between full expiry sweeps of the guard map.
const PRUNE_INTERVAL: Duration = Duration::from_secs(30);
/// Buckets inspected when an insertion has to evict at capacity.
const EVICTION_SAMPLE: usize = 64;
/// Cookie proving that this browser already authenticated in this process.
const KNOWN_CLIENT_COOKIE: &str = "rullst_nexus_known_client";
/// Lifetime of the known-client cookie; the value itself changes per process.
const KNOWN_CLIENT_MAX_AGE_SECS: u64 = 30 * 24 * 60 * 60;

#[derive(Debug, Clone, Copy)]
struct FailedAuthState {
    failures: u32,
    window_started: Instant,
    last_seen: Instant,
    locked_until: Option<Instant>,
}

struct GuardState {
    buckets: HashMap<IpAddr, FailedAuthState>,
    next_prune: Instant,
}

/// Failed-credential guard shared by every clone of one Basic Auth policy.
///
/// Failures are counted per client bucket. A locked bucket stops credential
/// evaluation for unknown clients, so the lockout cannot be used as a
/// password oracle. A browser that already authenticated in this process
/// carries a random known-client cookie and keeps having its credentials
/// evaluated, so a shared proxy address cannot lock it out.
pub(super) struct BasicAuthRateLimiter {
    state: Mutex<GuardState>,
    known_client_token: String,
    max_failures: u32,
    failure_window: Duration,
    lockout: Duration,
    max_buckets: usize,
}

impl Default for BasicAuthRateLimiter {
    fn default() -> Self {
        Self {
            state: Mutex::new(GuardState {
                buckets: HashMap::new(),
                next_prune: Instant::now(),
            }),
            known_client_token: rullst_core::security::generate_csrf_token(),
            max_failures: NEXUS_BASIC_AUTH_MAX_FAILURES,
            failure_window: NEXUS_BASIC_AUTH_FAILURE_WINDOW,
            lockout: NEXUS_BASIC_AUTH_LOCKOUT,
            max_buckets: NEXUS_BASIC_AUTH_MAX_PEERS,
        }
    }
}

impl fmt::Debug for BasicAuthRateLimiter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BasicAuthRateLimiter")
            .field("known_client_token", &"[REDACTED]")
            .field("max_failures", &self.max_failures)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum AuthGuardStatus {
    Allowed,
    Locked(Duration),
    Unavailable,
}

/// Maps a peer to its rate-limit bucket: the IPv4 address, or the /64 prefix
/// of an IPv6 address (IPv4-mapped IPv6 addresses count as IPv4).
pub(super) fn client_bucket(peer: IpAddr) -> IpAddr {
    match peer {
        IpAddr::V4(address) => IpAddr::V4(address),
        IpAddr::V6(address) => match address.to_ipv4_mapped() {
            Some(mapped) => IpAddr::V4(mapped),
            None => IpAddr::V6(Ipv6Addr::from(u128::from(address) & (u128::MAX << 64))),
        },
    }
}

impl BasicAuthRateLimiter {
    pub(super) fn status(&self, peer: IpAddr) -> AuthGuardStatus {
        self.status_at(client_bucket(peer), Instant::now())
    }

    pub(super) fn record_failure(&self, peer: IpAddr) -> AuthGuardStatus {
        self.record_failure_at(client_bucket(peer), Instant::now())
    }

    /// True when the request carries this process's known-client cookie.
    pub(super) fn is_known_client(&self, headers: &HeaderMap) -> bool {
        let expected = self.known_client_token.as_bytes();
        headers
            .get_all(header::COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .flat_map(|value| value.split(';'))
            .filter_map(|pair| pair.trim().split_once('='))
            .any(|(name, value)| {
                name == KNOWN_CLIENT_COOKIE
                    && value.len() == expected.len()
                    && bool::from(value.as_bytes().ct_eq(expected))
            })
    }

    /// `Set-Cookie` value marking a browser that presented valid credentials.
    pub(super) fn known_client_cookie(&self) -> Option<HeaderValue> {
        HeaderValue::from_str(&format!(
            "{KNOWN_CLIENT_COOKIE}={}; Path=/; Max-Age={KNOWN_CLIENT_MAX_AGE_SECS}; HttpOnly; Secure; SameSite=Lax",
            self.known_client_token
        ))
        .ok()
    }

    fn status_at(&self, bucket: IpAddr, now: Instant) -> AuthGuardStatus {
        let Ok(mut guard) = self.state.lock() else {
            return AuthGuardStatus::Unavailable;
        };
        guard.prune_if_due(now, self.failure_window);

        let Some(state) = guard.buckets.get_mut(&bucket) else {
            return AuthGuardStatus::Allowed;
        };
        let Some(locked_until) = state.locked_until else {
            return AuthGuardStatus::Allowed;
        };
        if locked_until > now {
            state.last_seen = now;
            AuthGuardStatus::Locked(locked_until.duration_since(now))
        } else {
            guard.buckets.remove(&bucket);
            AuthGuardStatus::Allowed
        }
    }

    fn record_failure_at(&self, bucket: IpAddr, now: Instant) -> AuthGuardStatus {
        let Ok(mut guard) = self.state.lock() else {
            return AuthGuardStatus::Unavailable;
        };
        guard.prune_if_due(now, self.failure_window);
        guard.ensure_capacity(bucket, self.max_buckets);

        let state = guard.buckets.entry(bucket).or_insert(FailedAuthState {
            failures: 0,
            window_started: now,
            last_seen: now,
            locked_until: None,
        });
        if now.duration_since(state.window_started) > self.failure_window {
            state.failures = 0;
            state.window_started = now;
        }
        state.failures = state.failures.saturating_add(1);
        state.last_seen = now;

        if state.failures >= self.max_failures {
            let Some(locked_until) = now.checked_add(self.lockout) else {
                return AuthGuardStatus::Unavailable;
            };
            state.locked_until = Some(locked_until);
            AuthGuardStatus::Locked(self.lockout)
        } else {
            AuthGuardStatus::Allowed
        }
    }
}

impl GuardState {
    /// Sweeps expired buckets at most once per [`PRUNE_INTERVAL`], so ordinary
    /// requests never scan the whole map under the lock.
    fn prune_if_due(&mut self, now: Instant, failure_window: Duration) {
        if now < self.next_prune {
            return;
        }
        self.buckets.retain(|_, state| {
            state.locked_until.is_some_and(|until| until > now)
                || now.duration_since(state.last_seen) <= failure_window
        });
        self.next_prune = now.checked_add(PRUNE_INTERVAL).unwrap_or(now);
    }

    /// Evicts the stalest of a bounded sample when a new bucket would exceed
    /// the cap, keeping the cost per failure independent of the map size.
    fn ensure_capacity(&mut self, incoming: IpAddr, max_buckets: usize) {
        if self.buckets.len() < max_buckets || self.buckets.contains_key(&incoming) {
            return;
        }
        if let Some(stalest) = self
            .buckets
            .iter()
            .take(EVICTION_SAMPLE)
            .min_by_key(|(_, state)| state.last_seen)
            .map(|(bucket, _)| *bucket)
        {
            self.buckets.remove(&stalest);
        }
    }
}

#[cfg(test)]
mod tests {
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
    }
}
