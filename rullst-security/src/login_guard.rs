//! Anti-Bruteforce Tarpit & Login Jail Security Engine.
//! Provides progressive async delay (tarpit) and temporary in-memory jail bans for repeated auth failures.

use crate::telemetry::{LiveSecurityEvent, SecurityStore, current_timestamp_str};
use dashmap::DashMap;
use std::sync::OnceLock;
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
}

impl Default for LoginGuard {
    fn default() -> Self {
        Self {
            failures: DashMap::new(),
            jails: DashMap::new(),
            max_failures: 5,
            jail_duration: Duration::from_secs(900), // 15 minutes
            window_duration: Duration::from_secs(600), // 10 minutes
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
        if let Some(exp) = self.jails.get(identity) {
            if Instant::now() < *exp {
                return true;
            } else {
                drop(exp);
                self.jails.remove(identity);
            }
        }
        false
    }

    /// Returns the remaining jail duration for an identity, if jailed.
    pub fn remaining_jail_time(&self, identity: &str) -> Option<Duration> {
        if let Some(exp) = self.jails.get(identity) {
            let now = Instant::now();
            if now < *exp {
                return Some(exp.duration_since(now));
            } else {
                drop(exp);
                self.jails.remove(identity);
            }
        }
        None
    }

    /// Records a failed authentication attempt. Returns the progressive tarpit delay duration.
    pub fn record_login_failure(&self, identity: &str) -> Duration {
        let now = Instant::now();

        // Check if already jailed
        if self.is_jailed(identity) {
            return Duration::from_secs(5);
        }

        let current_count = {
            let mut entry = self.failures.entry(identity.to_string()).or_insert((0, now));
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
            self.jails
                .insert(identity.to_string(), now + self.jail_duration);
            self.failures.remove(identity);

            // Record security telemetry & live event
            let store = SecurityStore::global();
            store.inc_login_jail_bans();

            if let Ok(mut events) = store.live_events.lock() {
                events.insert(
                    0,
                    LiveSecurityEvent {
                        event_type: "LOGIN_JAIL_TRIGGERED".to_string(),
                        details: format!(
                            "Identity/IP '{}' placed in 15min jail after {} failed login attempts",
                            identity, current_count
                        ),
                        client_ip: identity.to_string(),
                        timestamp_str: current_timestamp_str(),
                        verified_hmac: true,
                    },
                );
                if events.len() > 50 {
                    events.truncate(50);
                }
            }

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
        self.failures.remove(identity);
        self.jails.remove(identity);
    }
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
