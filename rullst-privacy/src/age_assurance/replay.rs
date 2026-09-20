use super::AgeError;
use std::{collections::BTreeMap, future::Future, sync::Mutex};

/// The host must substantiate a shared store's durability and atomicity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayDurability {
    ProcessLocal,
    SharedDurable,
}

/// Store implementations are trusted server integrations. Atomically insert a
/// nonce until `expires_at`; return false if already claimed. Claims must survive
/// restart and be shared across every production verifier/replica. Never evict
/// unexpired entries to make space. Uncertain commits must fail closed.
/// Timestamps are trusted server Unix seconds; positive remaining lifetime is
/// bounded to 900 seconds. Reject clock rollback before pruning expired claims.
/// Cancellation may leave a consumed nonce, but must never return permission.
///
/// Age evidence consumption and an application domain mutation are not one
/// transaction. The caller still owns domain idempotency and recovery.
pub trait ReplayStore: Send + Sync {
    fn durability(&self) -> ReplayDurability;

    fn claim(
        &self,
        nonce: [u8; 32],
        expires_at: i64,
        now: i64,
    ) -> impl Future<Output = Result<bool, AgeError>> + Send;
}

pub(super) fn validate_claim(expires_at: i64, now: i64) -> Result<(), AgeError> {
    if now < 0 || expires_at <= now || expires_at - now > 900 {
        return Err(AgeError::InvalidChallenge);
    }
    Ok(())
}

#[derive(Default)]
struct MemoryState {
    claims: BTreeMap<[u8; 32], i64>,
    last_now: i64,
}

/// Bounded offline/development store. Production verifier construction rejects it.
pub struct MemoryReplayStore {
    capacity: usize,
    state: Mutex<MemoryState>,
}

impl MemoryReplayStore {
    pub fn new(capacity: usize) -> Result<Self, AgeError> {
        if !(1..=100_000).contains(&capacity) {
            return Err(AgeError::InvalidConfiguration);
        }
        Ok(Self {
            capacity,
            state: Mutex::new(MemoryState::default()),
        })
    }
}

impl ReplayStore for MemoryReplayStore {
    fn durability(&self) -> ReplayDurability {
        ReplayDurability::ProcessLocal
    }

    async fn claim(&self, nonce: [u8; 32], expires_at: i64, now: i64) -> Result<bool, AgeError> {
        validate_claim(expires_at, now)?;
        let mut state = self.state.lock().map_err(|_| AgeError::StoreUnavailable)?;
        if now < state.last_now {
            return Err(AgeError::ClockRollback);
        }
        state.last_now = now;
        let claims = &mut state.claims;
        claims.retain(|_, expiry| *expiry > now);
        if claims.contains_key(&nonce) {
            return Ok(false);
        }
        if claims.len() >= self.capacity {
            return Err(AgeError::StoreCapacity);
        }
        claims.insert(nonce, expires_at);
        Ok(true)
    }
}

impl<T: ReplayStore> ReplayStore for std::sync::Arc<T> {
    fn durability(&self) -> ReplayDurability {
        (**self).durability()
    }

    async fn claim(&self, nonce: [u8; 32], expires_at: i64, now: i64) -> Result<bool, AgeError> {
        (**self).claim(nonce, expires_at, now).await
    }
}
