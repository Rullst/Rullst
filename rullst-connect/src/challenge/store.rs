//! The challenge store contract and its bounded in-memory implementation.

use super::{ChallengeError, ChallengeId, LoginChallenge};
use std::collections::HashMap;
use std::future::Future;
use std::sync::Mutex;

/// Largest entry limit a [`MemoryChallengeStore`] accepts.
const MAX_MEMORY_ENTRIES: usize = 1_000_000;

/// Storage for login challenges.
///
/// Implementations key each challenge by [`ChallengeId::storage_key`] and
/// must make [`Self::remove`] atomic: of any number of concurrent calls with
/// one id, at most one returns the challenge. [`Self::take`] builds the
/// single-use, expiry-checked consumption on top of it.
pub trait ChallengeStore: Send + Sync {
    /// Stores `challenge` under a fresh [`ChallengeId`] and returns that id.
    ///
    /// Refuses with [`ChallengeError::CapacityExceeded`] when the store holds
    /// its maximum of live challenges after dropping expired ones.
    fn save(
        &self,
        challenge: &LoginChallenge,
    ) -> impl Future<Output = Result<ChallengeId, ChallengeError>> + Send;

    /// Atomically deletes and returns the challenge stored under `id`,
    /// expired or not; `None` when there is none.
    fn remove(
        &self,
        id: &ChallengeId,
    ) -> impl Future<Output = Result<Option<LoginChallenge>, ChallengeError>> + Send;

    /// Deletes at most `limit` challenges expired at Unix second `now` and
    /// returns how many it deleted.
    fn purge_expired(
        &self,
        now: u64,
        limit: u32,
    ) -> impl Future<Output = Result<u64, ChallengeError>> + Send;

    /// Consumes the challenge exactly once: [`ChallengeError::NotFound`] when
    /// it is unknown or already consumed, [`ChallengeError::Expired`] (after
    /// deleting it) when it expired at Unix second `now`.
    fn take(
        &self,
        id: &ChallengeId,
        now: u64,
    ) -> impl Future<Output = Result<LoginChallenge, ChallengeError>> + Send {
        async move {
            match self.remove(id).await? {
                Some(challenge) if challenge.is_expired_at(now) => Err(ChallengeError::Expired),
                Some(challenge) => Ok(challenge),
                None => Err(ChallengeError::NotFound),
            }
        }
    }
}

/// A bounded challenge store inside one process, for tests and development.
///
/// Challenges do not survive a restart and are not shared between processes
/// or hosts; use a SQL store for a deployed application.
#[derive(Debug)]
pub struct MemoryChallengeStore {
    entries: Mutex<HashMap<String, LoginChallenge>>,
    max_entries: usize,
}

impl MemoryChallengeStore {
    /// A store holding at most `max_entries` (1 to 1,000,000) live challenges.
    pub fn new(max_entries: usize) -> Result<Self, ChallengeError> {
        if !(1..=MAX_MEMORY_ENTRIES).contains(&max_entries) {
            return Err(ChallengeError::Invalid("entry limit"));
        }
        Ok(Self {
            entries: Mutex::new(HashMap::new()),
            max_entries,
        })
    }

    /// The number of stored challenges, expired ones included.
    pub fn len(&self) -> Result<usize, ChallengeError> {
        Ok(self.lock()?.len())
    }

    /// Whether the store holds no challenge.
    pub fn is_empty(&self) -> Result<bool, ChallengeError> {
        Ok(self.lock()?.is_empty())
    }

    fn lock(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, HashMap<String, LoginChallenge>>, ChallengeError> {
        self.entries
            .lock()
            .map_err(|_| ChallengeError::StorageUnavailable("lock memory store"))
    }
}

impl ChallengeStore for MemoryChallengeStore {
    async fn save(&self, challenge: &LoginChallenge) -> Result<ChallengeId, ChallengeError> {
        let mut entries = self.lock()?;
        if entries.len() >= self.max_entries {
            let now = challenge.created_at();
            entries.retain(|_, stored| !stored.is_expired_at(now));
        }
        if entries.len() >= self.max_entries {
            return Err(ChallengeError::CapacityExceeded);
        }
        let id = ChallengeId::generate();
        entries.insert(id.storage_key(), challenge.clone());
        Ok(id)
    }

    async fn remove(&self, id: &ChallengeId) -> Result<Option<LoginChallenge>, ChallengeError> {
        Ok(self.lock()?.remove(&id.storage_key()))
    }

    async fn purge_expired(&self, now: u64, limit: u32) -> Result<u64, ChallengeError> {
        let mut entries = self.lock()?;
        let expired: Vec<String> = entries
            .iter()
            .filter(|(_, challenge)| challenge.is_expired_at(now))
            .map(|(key, _)| key.clone())
            .take(limit as usize)
            .collect();
        for key in &expired {
            entries.remove(key);
        }
        Ok(expired.len() as u64)
    }
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
