use std::{
    collections::HashMap,
    net::IpAddr,
    sync::Mutex,
    time::{Duration, Instant},
};

/// Number of failed requests allowed per peer before the default lockout starts.
pub const NEXUS_BASIC_AUTH_MAX_FAILURES: u32 = 5;
/// Window in which Basic Auth failures are accumulated.
pub const NEXUS_BASIC_AUTH_FAILURE_WINDOW: Duration = Duration::from_secs(5 * 60);
/// Default lockout duration after too many Basic Auth failures.
pub const NEXUS_BASIC_AUTH_LOCKOUT: Duration = Duration::from_secs(15 * 60);
/// Maximum number of peer identities retained by the Basic Auth guard.
pub const NEXUS_BASIC_AUTH_MAX_PEERS: usize = 100_000;

#[derive(Debug, Clone, Copy)]
struct FailedAuthState {
    failures: u32,
    window_started: Instant,
    last_seen: Instant,
    locked_until: Option<Instant>,
}

#[derive(Debug)]
pub(super) struct BasicAuthRateLimiter {
    peers: Mutex<HashMap<IpAddr, FailedAuthState>>,
    max_failures: u32,
    failure_window: Duration,
    lockout: Duration,
    max_peers: usize,
}

impl Default for BasicAuthRateLimiter {
    fn default() -> Self {
        Self {
            peers: Mutex::new(HashMap::new()),
            max_failures: NEXUS_BASIC_AUTH_MAX_FAILURES,
            failure_window: NEXUS_BASIC_AUTH_FAILURE_WINDOW,
            lockout: NEXUS_BASIC_AUTH_LOCKOUT,
            max_peers: NEXUS_BASIC_AUTH_MAX_PEERS,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum AuthGuardStatus {
    Allowed,
    Locked(Duration),
    Unavailable,
}

impl BasicAuthRateLimiter {
    pub(super) fn status(&self, peer: IpAddr) -> AuthGuardStatus {
        let now = Instant::now();
        let Ok(mut peers) = self.peers.lock() else {
            return AuthGuardStatus::Unavailable;
        };
        prune_auth_peers(&mut peers, now, self.failure_window);

        let Some(state) = peers.get_mut(&peer) else {
            return AuthGuardStatus::Allowed;
        };
        let Some(locked_until) = state.locked_until else {
            return AuthGuardStatus::Allowed;
        };
        if locked_until > now {
            state.last_seen = now;
            AuthGuardStatus::Locked(locked_until.duration_since(now))
        } else {
            peers.remove(&peer);
            AuthGuardStatus::Allowed
        }
    }

    pub(super) fn record_failure(&self, peer: IpAddr) -> AuthGuardStatus {
        let now = Instant::now();
        let Ok(mut peers) = self.peers.lock() else {
            return AuthGuardStatus::Unavailable;
        };
        prune_auth_peers(&mut peers, now, self.failure_window);
        ensure_auth_peer_capacity(&mut peers, peer, self.max_peers);

        let state = peers.entry(peer).or_insert(FailedAuthState {
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

    pub(super) fn record_success(&self, peer: IpAddr) -> bool {
        let Ok(mut peers) = self.peers.lock() else {
            return false;
        };
        peers.remove(&peer);
        true
    }
}

fn prune_auth_peers(
    peers: &mut HashMap<IpAddr, FailedAuthState>,
    now: Instant,
    failure_window: Duration,
) {
    peers.retain(|_, state| {
        state.locked_until.is_some_and(|until| until > now)
            || now.duration_since(state.last_seen) <= failure_window
    });
}

fn ensure_auth_peer_capacity(
    peers: &mut HashMap<IpAddr, FailedAuthState>,
    incoming_peer: IpAddr,
    max_peers: usize,
) {
    if peers.len() < max_peers || peers.contains_key(&incoming_peer) {
        return;
    }
    if let Some(oldest_peer) = peers
        .iter()
        .min_by_key(|(_, state)| state.last_seen)
        .map(|(peer, _)| *peer)
    {
        peers.remove(&oldest_peer);
    }
}
