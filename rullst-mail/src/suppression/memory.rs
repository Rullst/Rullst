use super::{
    MutableSuppressionStore, SuppressionError, SuppressionEvent, SuppressionRecord,
    SuppressionSnapshot, SuppressionStore, normalize_recipient, unavailable, validate_limits,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Bounded deterministic process-local suppression state.
#[derive(Clone)]
pub struct InMemorySuppressionStore {
    state: Arc<Mutex<MemoryState>>,
    max_recipients: usize,
    max_events: usize,
}

#[derive(Default)]
struct MemoryState {
    recipients: HashMap<String, SuppressionRecord>,
    events: HashMap<(String, String), EventFingerprint>,
}

#[derive(Clone, Eq, PartialEq)]
struct EventFingerprint {
    recipient: String,
    reason: super::SuppressionReason,
    observed_at: u64,
}

impl InMemorySuppressionStore {
    /// Creates a bounded process-local store.
    pub fn new(max_recipients: usize, max_events: usize) -> Result<Self, SuppressionError> {
        validate_limits(max_recipients, max_events)?;
        Ok(Self {
            state: Arc::new(Mutex::new(MemoryState::default())),
            max_recipients,
            max_events,
        })
    }

    /// Returns current bounded counts.
    pub fn snapshot(&self) -> Result<SuppressionSnapshot, SuppressionError> {
        let state = self.state.lock().map_err(|_| unavailable("lock state"))?;
        Ok(SuppressionSnapshot::new(
            state.recipients.len(),
            state.events.len(),
            self.max_recipients,
            self.max_events,
        ))
    }

    /// Removes replay identifiers older than `cutoff`, retaining suppressions.
    pub fn prune_events_before(&self, cutoff: u64) -> Result<usize, SuppressionError> {
        if cutoff == 0 {
            return Err(SuppressionError::InvalidConfiguration("event cutoff"));
        }
        let mut state = self.state.lock().map_err(|_| unavailable("lock state"))?;
        let before = state.events.len();
        state.events.retain(|_, event| event.observed_at >= cutoff);
        Ok(before.saturating_sub(state.events.len()))
    }
}

impl SuppressionStore for InMemorySuppressionStore {
    async fn lookup(&self, recipient: &str) -> Result<Option<SuppressionRecord>, SuppressionError> {
        let recipient = normalize_recipient(recipient)?;
        self.state
            .lock()
            .map_err(|_| unavailable("lock state"))
            .map(|state| state.recipients.get(&recipient).cloned())
    }
}

impl MutableSuppressionStore for InMemorySuppressionStore {
    async fn record(&self, event: SuppressionEvent) -> Result<SuppressionRecord, SuppressionError> {
        let mut state = self.state.lock().map_err(|_| unavailable("lock state"))?;
        let key = (event.provider.clone(), event.event_id.clone());
        let fingerprint = EventFingerprint {
            recipient: event.recipient.clone(),
            reason: event.reason,
            observed_at: event.observed_at,
        };
        if let Some(existing) = state.events.get(&key) {
            if existing != &fingerprint {
                return Err(SuppressionError::EventConflict);
            }
            return state
                .recipients
                .get(&event.recipient)
                .cloned()
                .ok_or(SuppressionError::CorruptStorage("event recipient"));
        }
        if state.events.len() >= self.max_events
            || (!state.recipients.contains_key(&event.recipient)
                && state.recipients.len() >= self.max_recipients)
        {
            return Err(SuppressionError::CapacityExceeded);
        }
        let record = merge_record(state.recipients.get(&event.recipient), &event);
        state
            .recipients
            .insert(event.recipient.clone(), record.clone());
        state.events.insert(key, fingerprint);
        Ok(record)
    }
}

fn merge_record(
    existing: Option<&SuppressionRecord>,
    event: &SuppressionEvent,
) -> SuppressionRecord {
    let Some(existing) = existing else {
        return SuppressionRecord {
            recipient: event.recipient.clone(),
            reason: event.reason,
            provider: event.provider.clone(),
            first_seen_at: event.observed_at,
            last_seen_at: event.observed_at,
        };
    };
    let authoritative = event.reason.rank() > existing.reason.rank()
        || (event.reason == existing.reason && event.observed_at >= existing.last_seen_at);
    SuppressionRecord {
        recipient: existing.recipient.clone(),
        reason: if authoritative {
            event.reason
        } else {
            existing.reason
        },
        provider: if authoritative {
            event.provider.clone()
        } else {
            existing.provider.clone()
        },
        first_seen_at: existing.first_seen_at.min(event.observed_at),
        last_seen_at: existing.last_seen_at.max(event.observed_at),
    }
}
