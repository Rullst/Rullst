use super::AgeError;
use std::{collections::BTreeMap, sync::Mutex};

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
///
/// Age evidence consumption and an application domain mutation are not one
/// transaction. The caller still owns domain idempotency and recovery.
pub trait ReplayStore: Send + Sync {
    fn durability(&self) -> ReplayDurability;

    fn claim(&self, nonce: [u8; 32], expires_at: i64, now: i64) -> Result<bool, AgeError>;
}

/// Bounded offline/development store. Production verifier construction rejects it.
pub struct MemoryReplayStore {
    capacity: usize,
    claims: Mutex<BTreeMap<[u8; 32], i64>>,
}

impl MemoryReplayStore {
    pub fn new(capacity: usize) -> Result<Self, AgeError> {
        if !(1..=100_000).contains(&capacity) {
            return Err(AgeError::InvalidConfiguration);
        }
        Ok(Self {
            capacity,
            claims: Mutex::new(BTreeMap::new()),
        })
    }
}

impl ReplayStore for MemoryReplayStore {
    fn durability(&self) -> ReplayDurability {
        ReplayDurability::ProcessLocal
    }

    fn claim(&self, nonce: [u8; 32], expires_at: i64, now: i64) -> Result<bool, AgeError> {
        if now < 0 || expires_at <= now {
            return Err(AgeError::InvalidChallenge);
        }
        let mut claims = self.claims.lock().map_err(|_| AgeError::StoreUnavailable)?;
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

    fn claim(&self, nonce: [u8; 32], expires_at: i64, now: i64) -> Result<bool, AgeError> {
        (**self).claim(nonce, expires_at, now)
    }
}
