//! Anti-Bruteforce Tarpit & Login Jail Security Engine.
//! Provides progressive async delay (tarpit) and temporary in-memory jail bans for repeated auth failures.
//!
//! Both the failure counters and the jails are bounded by
//! [`LoginGuard::max_identities`]. When the failure index is full, a new
//! identity evicts the counter with the oldest last attempt instead of being
//! ignored; when the jail index is full, a new offender evicts the jail that
//! expires soonest. A flood of unrelated identities can therefore shorten the
//! memory of old failures, but it cannot stop a newly observed identity from
//! being counted and jailed. Expired counters and jails are pruned on every
//! operation in time order.

mod state;

#[cfg(test)]
mod capacity_tests;

use crate::telemetry::{LiveSecurityEvent, SecurityStore};
use sha2::{Digest, Sha256};
use state::{IdentityKey, TimeOrderedIndex};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

static GLOBAL_LOGIN_GUARD: OnceLock<LoginGuard> = OnceLock::new();

/// Delay returned for jailed identities and for fail-closed states.
const JAILED_DELAY: Duration = Duration::from_secs(5);

/// Anti-Bruteforce Tarpit and Login Jail Engine.
pub struct LoginGuard {
    /// Failure counters ordered by last attempt and active jails ordered by expiry.
    state: Mutex<LoginState>,
    /// Max failures allowed before triggering temporary jail (default: 5).
    pub max_failures: u32,
    /// Duration of the temporary jail ban (default: 15 minutes).
    pub jail_duration: Duration,
    /// Reset window for consecutive failures (default: 10 minutes).
    pub window_duration: Duration,
    /// Maximum identities retained in either in-memory map.
    ///
    /// At capacity the least recently failed counter, or the jail that expires
    /// soonest, is evicted so that a new identity is still tracked. A value of
    /// zero stores nothing and returns the jailed delay for every failure.
    pub max_identities: usize,
}

struct LoginState {
    /// Consecutive failure counts, ordered by last attempt.
    failures: TimeOrderedIndex<u32>,
    /// Active temporary bans, ordered by expiration.
    jails: TimeOrderedIndex<()>,
}

impl LoginState {
    fn prune_expired(&mut self, now: Instant, window: Duration) {
        self.failures
            .prune_while(|last_attempt| now.saturating_duration_since(last_attempt) >= window);
        self.jails.prune_while(|expires_at| expires_at <= now);
    }
}

impl Default for LoginGuard {
    fn default() -> Self {
        Self {
            state: Mutex::new(LoginState {
                failures: TimeOrderedIndex::new(),
                jails: TimeOrderedIndex::new(),
            }),
            max_failures: 5,
            jail_duration: Duration::from_secs(900), // 15 minutes
            window_duration: Duration::from_secs(600), // 10 minutes
            max_identities: 100_000,
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
        let Ok(mut state) = self.state.lock() else {
            return true;
        };
        state.prune_expired(Instant::now(), self.window_duration);
        state.jails.contains(&identity_key(identity))
    }

    /// Returns the remaining jail duration for an identity, if jailed.
    pub fn remaining_jail_time(&self, identity: &str) -> Option<Duration> {
        let Ok(mut state) = self.state.lock() else {
            return Some(JAILED_DELAY);
        };
        let now = Instant::now();
        state.prune_expired(now, self.window_duration);
        state
            .jails
            .get(&identity_key(identity))
            .map(|(expires_at, _)| expires_at.saturating_duration_since(now))
    }

    /// Records a failed authentication attempt. Returns the progressive tarpit delay duration.
    ///
    /// A new identity is always counted: at capacity the counter with the
    /// oldest last attempt is evicted. Reaching [`Self::max_failures`] always
    /// creates a jail, evicting the jail that expires soonest when the jail
    /// index is full, and only then clears the identity's failure counter.
    pub fn record_login_failure(&self, identity: &str) -> Duration {
        let Ok(mut state) = self.state.lock() else {
            return JAILED_DELAY;
        };
        if self.max_identities == 0 {
            return JAILED_DELAY;
        }
        let now = Instant::now();
        state.prune_expired(now, self.window_duration);
        let identity_key = identity_key(identity);

        // Check if already jailed
        if state.jails.contains(&identity_key) {
            return JAILED_DELAY;
        }

        let current_count = match state.failures.get(&identity_key) {
            Some((last_attempt, count))
                if now.saturating_duration_since(last_attempt) <= self.window_duration =>
            {
                count.saturating_add(1)
            }
            // Missing or beyond the window: restart the sequence.
            _ => 1,
        };

        if current_count >= self.max_failures {
            let expires_at = saturating_deadline(now, self.jail_duration);
            if expires_at > now {
                state.jails.make_room(self.max_identities);
                state.jails.upsert(identity_key, expires_at, ());
                state.failures.remove(&identity_key);
                drop(state);
                record_jail_telemetry(identity, self.jail_duration, current_count);
                return JAILED_DELAY;
            }
            // A zero-length jail cannot be created; keep counting instead.
            track_failure(
                &mut state,
                identity_key,
                now,
                current_count,
                self.max_identities,
            );
            return JAILED_DELAY;
        }

        track_failure(
            &mut state,
            identity_key,
            now,
            current_count,
            self.max_identities,
        );
        // Progressive tarpit delay: 1st=0s, 2nd=1s, 3rd=2s, 4th=4s
        Duration::from_secs(progressive_delay_seconds(current_count))
    }

    /// Records a failed login and applies the returned progressive delay.
    ///
    /// Prefer this method in authentication handlers so the tarpit cannot be
    /// accidentally reduced to a duration that the caller forgets to await.
    pub async fn record_login_failure_and_wait(&self, identity: &str) -> Duration {
        let delay = self.record_login_failure(identity);
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }
        delay
    }

    /// Records a successful authentication, resetting the failure history.
    pub fn record_login_success(&self, identity: &str) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        let identity_key = identity_key(identity);
        state.failures.remove(&identity_key);
        state.jails.remove(&identity_key);
    }
}

/// Stores a failure counter, evicting the least recently failed identity when full.
fn track_failure(
    state: &mut LoginState,
    identity_key: IdentityKey,
    now: Instant,
    count: u32,
    capacity: usize,
) {
    if !state.failures.contains(&identity_key) {
        state.failures.make_room(capacity);
    }
    state.failures.upsert(identity_key, now, count);
}

fn record_jail_telemetry(identity: &str, jail_duration: Duration, current_count: u32) {
    let store = SecurityStore::global();
    store.inc_login_jail_bans();
    store.push_local_event(LiveSecurityEvent::local(
        "LOGIN_JAIL_TRIGGERED",
        format!(
            "Identity/IP '{}' placed in a {}s jail after {} failed login attempts",
            bounded_identity_for_log(identity),
            jail_duration.as_secs(),
            current_count
        ),
        bounded_identity_for_log(identity),
    ));
}

/// Returns `now + duration`, shortening a duration the clock cannot represent.
fn saturating_deadline(now: Instant, duration: Duration) -> Instant {
    let mut duration = duration;
    loop {
        if let Some(deadline) = now.checked_add(duration) {
            return deadline;
        }
        duration /= 2;
    }
}

const fn progressive_delay_seconds(current_count: u32) -> u64 {
    match current_count {
        1 => 0,
        2 => 1,
        3 => 2,
        _ => 4,
    }
}

fn identity_key(identity: &str) -> IdentityKey {
    Sha256::digest(identity.trim().as_bytes()).into()
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

    impl LoginGuard {
        pub(super) fn failure_count(&self) -> usize {
            self.state.lock().unwrap().failures.len()
        }

        pub(super) fn jail_count(&self) -> usize {
            self.state.lock().unwrap().jails.len()
        }

        pub(super) fn insert_jail(&self, identity: &str, expires_at: Instant) {
            self.state
                .lock()
                .unwrap()
                .jails
                .upsert(identity_key(identity), expires_at, ());
        }

        pub(super) fn insert_failure(&self, identity: &str, count: u32, last_attempt: Instant) {
            self.state
                .lock()
                .unwrap()
                .failures
                .upsert(identity_key(identity), last_attempt, count);
        }
    }

    pub(super) fn past(duration: Duration) -> Instant {
        Instant::now().checked_sub(duration).unwrap()
    }

    #[test]
    fn concurrent_identity_admission_preserves_the_failure_and_jail_limits() {
        for max_failures in [1, 5] {
            for _ in 0..16 {
                let guard = LoginGuard {
                    max_identities: 1,
                    max_failures,
                    ..LoginGuard::default()
                };
                let barrier = std::sync::Barrier::new(16);
                std::thread::scope(|scope| {
                    for index in 0..16 {
                        let guard = &guard;
                        let barrier = &barrier;
                        scope.spawn(move || {
                            barrier.wait();
                            guard.record_login_failure(&format!("concurrent-{index}"));
                        });
                    }
                });
                assert!(guard.failure_count() <= 1);
                assert!(guard.jail_count() <= 1);
            }
        }
    }

    #[test]
    // TM-AUTH-07: repeated failures receive bounded delay and temporary jailing.
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
        guard.insert_jail("expired_user", past(Duration::from_secs(10)));
        assert!(!guard.is_jailed("expired_user"));
        assert!(guard.remaining_jail_time("expired_user").is_none());

        // Insert active jail
        guard.insert_jail("active_user", Instant::now() + Duration::from_secs(100));
        assert!(guard.is_jailed("active_user"));
        assert!(guard.remaining_jail_time("active_user").is_some());
    }

    #[test]
    fn already_jailed_and_capacity_exhaustion_fail_closed() {
        let guard = LoginGuard::new();
        guard.insert_jail("jailed-user", Instant::now() + Duration::from_secs(60));
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
        assert_eq!(full_guard.failure_count(), 0);
        assert_eq!(full_guard.jail_count(), 0);
    }

    #[test]
    fn expired_failure_window_restarts_the_tarpit_sequence() {
        let mut guard = LoginGuard::new();
        guard.window_duration = Duration::from_millis(1);
        guard.insert_failure("window-user", 4, past(Duration::from_secs(1)));
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

    #[tokio::test(start_paused = true)]
    async fn async_failure_api_applies_the_progressive_delay() {
        let guard = LoginGuard::new();
        assert_eq!(
            guard.record_login_failure_and_wait("async-user").await,
            Duration::ZERO
        );
        assert_eq!(
            guard.record_login_failure_and_wait("async-user").await,
            Duration::from_secs(1)
        );
    }
}

#[cfg(kani)]
#[cfg_attr(mutants, mutants::skip)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    fn proof_progressive_delay_is_bounded() {
        let current_count: u32 = kani::any();
        let delay = progressive_delay_seconds(current_count);

        assert!(delay <= 4);
        match current_count {
            1 => assert_eq!(delay, 0),
            2 => assert_eq!(delay, 1),
            3 => assert_eq!(delay, 2),
            _ => assert_eq!(delay, 4),
        }
    }
}
