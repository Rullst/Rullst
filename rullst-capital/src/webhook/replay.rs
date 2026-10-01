//! Process-local bounded replay store and the replay backend selected by the
//! webhook middleware.

#[cfg(feature = "webhook-sql")]
use super::SqlWebhookReplayStore;
use super::{
    MAX_REPLAY_CAPACITY, MAX_REPLAY_TTL, MAX_WEBHOOK_PAYLOAD_BYTES, event_key_hash, payload_key,
    validate_replay_event_key, validate_replay_provider,
};
use crate::error::CapitalError;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const DEFAULT_REPLAY_CAPACITY: usize = 10_000;
const DEFAULT_REPLAY_TTL: Duration = Duration::from_secs(24 * 60 * 60);

pub(super) struct ReplayEntry {
    key: String,
    accepted_at: Instant,
}

/// Bounded in-memory webhook replay store with time-based eviction.
///
/// Multi-process applications can select `SqlWebhookReplayStore` through the
/// `webhook-sql` feature. This process-local variant fails closed at capacity
/// rather than evicting an active replay proof.
pub struct InMemoryWebhookReplayStore {
    pub(super) entries: Mutex<VecDeque<ReplayEntry>>,
    max_entries: usize,
    ttl: Duration,
}

// Never lock or print the ledger, which can hold a million replay keys.
impl std::fmt::Debug for InMemoryWebhookReplayStore {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InMemoryWebhookReplayStore")
            .field("max_entries", &self.max_entries)
            .field("ttl", &self.ttl)
            .finish_non_exhaustive()
    }
}

impl InMemoryWebhookReplayStore {
    /// Creates a replay store. Capacity and TTL must both be greater than zero.
    pub fn new(max_entries: usize, ttl: Duration) -> Result<Self, CapitalError> {
        if max_entries == 0 || max_entries > MAX_REPLAY_CAPACITY {
            return Err(CapitalError::ConfigurationError(
                "Webhook replay capacity must be between 1 and 1000000".to_string(),
            ));
        }
        if ttl.is_zero() || ttl > MAX_REPLAY_TTL {
            return Err(CapitalError::ConfigurationError(
                "Webhook replay TTL must be between 1 second and 30 days".to_string(),
            ));
        }

        Ok(Self {
            entries: Mutex::new(VecDeque::with_capacity(max_entries.min(1_024))),
            max_entries,
            ttl,
        })
    }

    /// Atomically rejects a replay or records a newly verified payload key.
    pub fn check_and_record(&self, key: impl Into<String>) -> Result<(), CapitalError> {
        let key = key.into();
        let valid = !key.is_empty()
            && key.len() <= 128
            && key
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'));
        if !valid {
            return Err(CapitalError::ConfigurationError(
                "Webhook replay key must use 1-128 ASCII letters, digits, dots, hyphens, or underscores"
                    .to_string(),
            ));
        }
        self.check_and_record_at(key, Instant::now())
    }

    pub(super) fn check_and_record_at(
        &self,
        key: String,
        now: Instant,
    ) -> Result<(), CapitalError> {
        let mut entries = self.entries.lock().map_err(|_| {
            CapitalError::General("Webhook replay store lock was poisoned".to_string())
        })?;

        while entries.front().is_some_and(|entry| {
            now.checked_duration_since(entry.accepted_at)
                .is_some_and(|age| age >= self.ttl)
        }) {
            let _ = entries.pop_front();
        }

        if entries.iter().any(|entry| entry.key == key) {
            return Err(CapitalError::WebhookReplay(key));
        }

        if entries.len() >= self.max_entries {
            return Err(CapitalError::WebhookReplayStoreFull);
        }
        entries.push_back(ReplayEntry {
            key,
            accepted_at: now,
        });
        Ok(())
    }

    /// Computes and records a stable provider-scoped SHA-256 payload key.
    pub fn record_payload(&self, provider_name: &str, payload: &[u8]) -> Result<(), CapitalError> {
        validate_replay_provider(provider_name)?;
        if payload.is_empty() || payload.len() > MAX_WEBHOOK_PAYLOAD_BYTES {
            return Err(CapitalError::ConfigurationError(
                "Webhook replay payload must contain 1 byte through the configured body limit"
                    .to_string(),
            ));
        }
        self.check_and_record(payload_key(provider_name, payload))
    }

    /// Records a provider-scoped semantic event identifier selected after signature verification.
    pub fn record_event_key(
        &self,
        provider_name: &str,
        event_key: &str,
    ) -> Result<(), CapitalError> {
        validate_replay_provider(provider_name)?;
        validate_replay_event_key(event_key)?;
        self.check_and_record(event_key_hash(provider_name, event_key))
    }
}

impl Default for InMemoryWebhookReplayStore {
    fn default() -> Self {
        Self {
            entries: Mutex::new(VecDeque::with_capacity(1_024)),
            max_entries: DEFAULT_REPLAY_CAPACITY,
            ttl: DEFAULT_REPLAY_TTL,
        }
    }
}

/// Explicit replay backend used by framework middleware.
#[derive(Clone)]
#[non_exhaustive]
pub enum WebhookReplayBackend {
    /// Process-local bounded replay protection.
    Memory(Arc<InMemoryWebhookReplayStore>),
    /// Durable relational replay protection shared by multiple processes.
    #[cfg(feature = "webhook-sql")]
    Sql(Arc<SqlWebhookReplayStore>),
}

impl std::fmt::Debug for WebhookReplayBackend {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Memory(store) => formatter.debug_tuple("Memory").field(store).finish(),
            #[cfg(feature = "webhook-sql")]
            Self::Sql(store) => formatter.debug_tuple("Sql").field(store).finish(),
        }
    }
}

impl From<Arc<InMemoryWebhookReplayStore>> for WebhookReplayBackend {
    fn from(store: Arc<InMemoryWebhookReplayStore>) -> Self {
        Self::Memory(store)
    }
}

#[cfg(feature = "webhook-sql")]
impl From<Arc<SqlWebhookReplayStore>> for WebhookReplayBackend {
    fn from(store: Arc<SqlWebhookReplayStore>) -> Self {
        Self::Sql(store)
    }
}

impl WebhookReplayBackend {
    #[cfg(any(feature = "axum", feature = "actix", test))]
    pub(super) async fn record_payload(
        &self,
        provider: &str,
        payload: &[u8],
    ) -> Result<(), CapitalError> {
        match self {
            Self::Memory(store) => store.record_payload(provider, payload),
            #[cfg(feature = "webhook-sql")]
            Self::Sql(store) => store.check_and_record_payload(provider, payload).await,
        }
    }
}
