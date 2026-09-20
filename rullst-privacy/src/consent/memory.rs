use super::*;
use std::{collections::BTreeMap, sync::Mutex};

#[derive(Default)]
struct State {
    rows: BTreeMap<(ConsentSubject, String), ConsentRecord>,
    last_now: i64,
}

/// Bounded development state. Production construction rejects this store.
pub struct MemoryConsentStore {
    capacity: usize,
    state: Mutex<State>,
}

impl MemoryConsentStore {
    pub fn new(capacity: usize) -> Result<Self, ConsentError> {
        if !(1..=100_000).contains(&capacity) {
            return Err(ConsentError::InvalidConfiguration);
        }
        Ok(Self {
            capacity,
            state: Mutex::new(State::default()),
        })
    }
}

impl ConsentStore for MemoryConsentStore {
    fn durability(&self) -> ConsentDurability {
        ConsentDurability::ProcessLocal
    }

    async fn read(
        &self,
        subject: &ConsentSubject,
        purpose_id: &str,
        now: i64,
    ) -> Result<ConsentRecord, ConsentError> {
        let absent = ConsentRecord::absent(subject.clone(), purpose_id)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| ConsentError::StoreUnavailable)?;
        if now < state.last_now {
            return Err(ConsentError::ClockRollback);
        }
        state.last_now = now;
        Ok(state
            .rows
            .get(&(subject.clone(), purpose_id.to_owned()))
            .cloned()
            .unwrap_or(absent))
    }

    async fn update(&self, update: &ConsentUpdate) -> Result<ConsentRecord, ConsentError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| ConsentError::StoreUnavailable)?;
        if update.now() < state.last_now {
            return Err(ConsentError::ClockRollback);
        }
        let key = (update.subject().clone(), update.purpose().id().to_owned());
        let absent = ConsentRecord::absent(key.0.clone(), key.1.clone())?;
        let current = state.rows.get(&key).unwrap_or(&absent);
        if current.revision() == 0 && state.rows.len() >= self.capacity {
            return Err(ConsentError::StoreCapacity);
        }
        let next = update.apply_to(current)?;
        state.last_now = update.now();
        state.rows.insert(key, next.clone());
        Ok(next)
    }
}
