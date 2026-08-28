//! Anti-Bruteforce Tarpit & Login Jail Security Engine.
//! Provides progressive async delay (tarpit) and temporary in-memory jail bans for repeated auth failures.

use crate::telemetry::{LiveSecurityEvent, SecurityStore};
use dashmap::DashMap;
use sha2::{Digest, Sha256};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

static GLOBAL_LOGIN_GUARD: OnceLock<LoginGuard> = OnceLock::new();

/// Anti-Bruteforce Tarpit and Login Jail Engine.
pub struct LoginGuard {
    /// Tracks consecutive failure counts and last attempt: identity -> (count, timestamp).
    failures: DashMap<String, (u32, Instant)>,
    /// Tracks active temporary bans: identity -> jail expiration timestamp.
    jails: DashMap<String, Instant>,
    /// Max failures allowed before triggering temporary jail (default: 5).
    pub max_failures: u32,
    /// Duration of the temporary jail ban (default: 15 minutes).
    pub jail_duration: Duration,
    /// Reset window for consecutive failures (default: 10 minutes).
    pub window_duration: Duration,
    /// Maximum identities retained in either in-memory map.
    pub max_identities: usize,
    last_cleanup: Mutex<Instant>,
}

impl Default for LoginGuard {
    fn default() -> Self {
        Self {
            failures: DashMap::new(),
            jails: DashMap::new(),
            max_failures: 5,
            jail_duration: Duration::from_secs(900), // 15 minutes
            window_duration: Duration::from_secs(600), // 10 minutes
            max_identities: 100_000,
            last_cleanup: Mutex::new(Instant::now()),
        }
    }
}

impl LoginGuard {
    /// Creates a new LoginGuard instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Accesses the global static LoginGuard instance.
    pub fn global() -> &'static LoginGuard {
        GLOBAL_LOGIN_GUARD.get_or_init(LoginGuard::new)
    }

    /// Checks if a client IP or user identity is currently jailed.
    pub fn is_jailed(&self, identity: &str) -> bool {
        self.cleanup_if_due();
        let identity_key = identity_key(identity);
        if let Some(exp) = self.jails.get(&identity_key) {
            if Instant::now() < *exp {
                return true;
            } else {
                drop(exp);
                self.jails.remove(&identity_key);
            }
        }
        false
    }

    /// Returns the remaining jail duration for an identity, if jailed.
    pub fn remaining_jail_time(&self, identity: &str) -> Option<Duration> {
        self.cleanup_if_due();
        let identity_key = identity_key(identity);
        if let Some(exp) = self.jails.get(&identity_key) {
            let now = Instant::now();
            if now < *exp {
                return Some(exp.duration_since(now));
            } else {
                drop(exp);
                self.jails.remove(&identity_key);
            }
        }
        None
    }

    /// Records a failed authentication attempt. Returns the progressive tarpit delay duration.
    pub fn record_login_failure(&self, identity: &str) -> Duration {
        self.cleanup_if_due();
        let now = Instant::now();
        let identity_key = identity_key(identity);

        // Check if already jailed
        if self.is_jailed(identity) {
            return Duration::from_secs(5);
        }

        if !self.failures.contains_key(&identity_key) && self.failures.len() >= self.max_identities
        {
            return Duration::from_secs(5);
        }

        let current_count = {
            let mut entry = self
                .failures
                .entry(identity_key.clone())
                .or_insert((0, now));
            let (count, last_attempt) = entry.value_mut();

            // Reset if beyond window
            if now.duration_since(*last_attempt) > self.window_duration {
                *count = 1;
                *last_attempt = now;
                1
            } else {
                *count += 1;
                *last_attempt = now;
                *count
            }
        };

        if current_count >= self.max_failures {
            self.failures.remove(&identity_key);
            if self.jails.len() < self.max_identities || self.jails.contains_key(&identity_key) {
                self.jails
                    .insert(identity_key.clone(), now + self.jail_duration);
            }

            // Record security telemetry & live event
            let store = SecurityStore::global();
            store.inc_login_jail_bans();

            store.push_local_event(LiveSecurityEvent::local(
                "LOGIN_JAIL_TRIGGERED",
                format!(
                    "Identity/IP '{}' placed in 15min jail after {} failed login attempts",
                    bounded_identity_for_log(identity),
                    current_count
                ),
                bounded_identity_for_log(identity),
            ));

            return Duration::from_secs(5);
        }

        // Progressive tarpit delay: 1st=0s, 2nd=1s, 3rd=2s, 4th=4s
        match current_count {
            1 => Duration::ZERO,
            2 => Duration::from_secs(1),
            3 => Duration::from_secs(2),
            _ => Duration::from_secs(4),
        }
    }

    /// Records a successful authentication, resetting the failure history.
    pub fn record_login_success(&self, identity: &str) {
        let identity_key = identity_key(identity);
        self.failures.remove(&identity_key);
        self.jails.remove(&identity_key);
    }

    fn cleanup_if_due(&self) {
        const CLEANUP_INTERVAL: Duration = Duration::from_secs(300);
        let now = Instant::now();
        let mut last_cleanup = match self.last_cleanup.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if now.duration_since(*last_cleanup) < CLEANUP_INTERVAL {
            return;
        }
        *last_cleanup = now;
        self.failures.retain(|_, (_, last_attempt)| {
            now.saturating_duration_since(*last_attempt) < self.window_duration
        });
        self.jails.retain(|_, expiration| now < *expiration);
    }
}

fn identity_key(identity: &str) -> String {
    hex::encode(Sha256::digest(identity.trim().as_bytes()))
}

fn bounded_identity_for_log(identity: &str) -> String {
    const MAX_LOGGED_IDENTITY_BYTES: usize = 128;
    let identity = identity.trim();
    if identity.len() <= MAX_LOGGED_IDENTITY_BYTES {
        return identity.to_string();
    }
    let mut boundary = MAX_LOGGED_IDENTITY_BYTES;
    while !identity.is_char_boundary(boundary) {
        boundary -= 1;
    }
    format!("{}…", &identity[..boundary])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_guard_tarpit_and_jail() {
        let guard = LoginGuard::new();
        let ip = "192.168.10.45";

        assert_eq!(guard.record_login_failure(ip), Duration::ZERO);
        assert_eq!(guard.record_login_failure(ip), Duration::from_secs(1));
        assert_eq!(guard.record_login_failure(ip), Duration::from_secs(2));
        assert_eq!(guard.record_login_failure(ip), Duration::from_secs(4));

        // 5th failure triggers jail
        assert_eq!(guard.record_login_failure(ip), Duration::from_secs(5));
        assert!(guard.is_jailed(ip));
        assert!(guard.remaining_jail_time(ip).is_some());

        // Reset on success
        guard.record_login_success(ip);
        assert!(!guard.is_jailed(ip));
    }

    #[test]
    fn test_login_guard_global_and_expired_jail() {
        let global = LoginGuard::global();
        assert_eq!(global.max_failures, 5);

        let guard = LoginGuard::new();
        // Insert expired jail
        guard.jails.insert(
            identity_key("expired_user"),
            Instant::now() - Duration::from_secs(10),
        );
        assert!(!guard.is_jailed("expired_user"));
        assert!(guard.remaining_jail_time("expired_user").is_none());

        // Insert active jail
        guard.jails.insert(
            identity_key("active_user"),
            Instant::now() + Duration::from_secs(100),
        );
        assert!(guard.is_jailed("active_user"));
        assert!(guard.remaining_jail_time("active_user").is_some());
    }

    #[test]
    fn already_jailed_and_capacity_exhaustion_fail_closed() {
        let guard = LoginGuard::new();
        guard.jails.insert(
            identity_key("jailed-user"),
            Instant::now() + Duration::from_secs(60),
        );
        assert_eq!(
            guard.record_login_failure("jailed-user"),
            Duration::from_secs(5)
        );

        let mut full_guard = LoginGuard::new();
        full_guard.max_identities = 0;
        assert_eq!(
            full_guard.record_login_failure("new-user"),
            Duration::from_secs(5)
        );
        assert!(full_guard.failures.is_empty());
    }

    #[test]
    fn expired_failure_window_restarts_the_tarpit_sequence() {
        let mut guard = LoginGuard::new();
        guard.window_duration = Duration::from_millis(1);
        guard.failures.insert(
            identity_key("window-user"),
            (4, Instant::now() - Duration::from_secs(1)),
        );
        assert_eq!(guard.record_login_failure("window-user"), Duration::ZERO);
    }

    #[test]
    fn logged_identity_is_trimmed_and_truncated_on_utf8_boundary() {
        assert_eq!(
            bounded_identity_for_log("  short identity  "),
            "short identity"
        );
        let long = format!("{}é-tail", "a".repeat(127));
        let bounded = bounded_identity_for_log(&long);
        assert!(bounded.ends_with('…'));
        assert!(bounded.len() <= 131);
        assert!(!bounded.contains("tail"));
    }
}

#[cfg(kani)]
#[cfg_attr(mutants, mutants::skip)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    fn proof_login_guard_is_jailed_initially_false() {
        let guard = LoginGuard::new();
        assert!(!guard.is_jailed("test_user_initial"));
    }
}
