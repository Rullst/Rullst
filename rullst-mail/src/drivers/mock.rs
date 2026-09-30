//! Deterministic, process-local fallback used by providers with mock credentials.

use crate::error::MailError;
use crate::message::Message;
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

/// Newest offline deliveries retained for inspection; older ones are evicted.
const MAX_OFFLINE_DELIVERIES: usize = 1_000;
/// Retained subject, body and attachment bytes across those deliveries.
const MAX_OFFLINE_BYTES: usize = 64 * 1024 * 1024;

static OFFLINE_DELIVERIES: OnceLock<Mutex<OfflineStore>> = OnceLock::new();
static OFFLINE_MODE_REPORTED: AtomicBool = AtomicBool::new(false);

/// Bounded FIFO of captured deliveries. The fallback is also reachable by
/// misconfiguration (an empty provider secret), so it must not grow forever.
#[derive(Default)]
struct OfflineStore {
    deliveries: VecDeque<(usize, OfflineMockDelivery)>,
    bytes: usize,
}

impl OfflineStore {
    /// Appends a delivery, then evicts the oldest ones until both bounds hold
    /// again. The newest delivery is always kept.
    fn push(&mut self, delivery: OfflineMockDelivery, max_items: usize, max_bytes: usize) {
        let size = retained_bytes(&delivery.message);
        self.bytes = self.bytes.saturating_add(size);
        self.deliveries.push_back((size, delivery));
        while self.deliveries.len() > 1
            && (self.deliveries.len() > max_items || self.bytes > max_bytes)
        {
            if let Some((evicted, _)) = self.deliveries.pop_front() {
                self.bytes = self.bytes.saturating_sub(evicted);
            }
        }
    }
}

fn retained_bytes(message: &Message) -> usize {
    let bodies = [message.body_html.as_deref(), message.body_text.as_deref()]
        .into_iter()
        .flatten()
        .map(str::len);
    let attachments = message.attachments.iter().map(|item| item.content.len());
    bodies
        .chain(attachments)
        .fold(message.subject.len(), usize::saturating_add)
}

/// Whether a provider will use its real transport or the deterministic offline fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeliveryMode {
    /// The configured credential selects the real external transport.
    Real,
    /// Empty or `mock_*` credentials select an in-memory transport with no network I/O.
    OfflineMock,
}

/// A delivery captured by an external provider's automatic offline fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct OfflineMockDelivery {
    /// Stable provider identifier such as `resend` or `sendgrid`.
    pub provider: String,
    /// Deterministic SHA-256 identifier derived from provider and sanitized message.
    pub delivery_id: String,
    /// The sanitized message which would have been sent.
    pub message: Message,
}

/// Inspector for provider-level offline fallback deliveries.
pub struct OfflineMailMock;

impl OfflineMailMock {
    /// Removes all captured offline deliveries.
    pub fn clear() -> Result<(), MailError> {
        let mut store = deliveries()
            .lock()
            .map_err(|_| MailError::DriverError("offline mock store lock poisoned".to_string()))?;
        store.deliveries.clear();
        store.bytes = 0;
        Ok(())
    }

    /// Returns a snapshot of the retained offline deliveries, oldest first.
    ///
    /// Only the newest 1,000 deliveries, and at most 64 MiB of their subject,
    /// body and attachment bytes, are retained; older ones are evicted.
    pub fn deliveries() -> Result<Vec<OfflineMockDelivery>, MailError> {
        deliveries()
            .lock()
            .map(|store| {
                store
                    .deliveries
                    .iter()
                    .map(|(_, delivery)| delivery.clone())
                    .collect()
            })
            .map_err(|_| MailError::DriverError("offline mock store lock poisoned".to_string()))
    }
}

/// Determines the transport mode from the documented credential convention.
pub fn credential_mode(credential: &str) -> DeliveryMode {
    let credential = credential.trim();
    if credential.is_empty() || credential.to_ascii_lowercase().starts_with("mock_") {
        DeliveryMode::OfflineMock
    } else {
        DeliveryMode::Real
    }
}

pub(crate) fn validate_credential(label: &str, credential: &str) -> Result<(), MailError> {
    if credential.contains(['\r', '\n']) {
        return Err(MailError::ConfigError(format!(
            "{label} contains forbidden CR/LF characters"
        )));
    }
    if credential.len() > 4096 {
        return Err(MailError::ConfigError(format!(
            "{label} exceeds the 4096-byte safety limit"
        )));
    }
    Ok(())
}

pub(crate) fn record_offline_delivery(provider: &str, message: &Message) -> Result<(), MailError> {
    let serialized = serde_json::to_vec(message).map_err(|error| {
        MailError::DriverError(format!("failed to serialize offline delivery: {error}"))
    })?;
    let mut hasher = Sha256::new();
    hasher.update(b"rullst-mail:offline-delivery:v1\0");
    hasher.update(provider.as_bytes());
    hasher.update(b"\0");
    hasher.update(serialized);
    let delivery_id = to_hex(&hasher.finalize());
    let delivery = OfflineMockDelivery {
        provider: provider.to_string(),
        delivery_id,
        message: message.clone(),
    };
    // An empty provider secret selects this fallback too, so make it visible
    // once per process instead of silently reporting success.
    if !OFFLINE_MODE_REPORTED.swap(true, Ordering::Relaxed) {
        tracing::warn!(
            event = "mail.offline_mock.active",
            provider,
            "Mail is captured by the offline mock (empty or mock_* credential) and not delivered"
        );
    }
    deliveries()
        .lock()
        .map_err(|_| MailError::DriverError("offline mock store lock poisoned".to_string()))?
        .push(delivery, MAX_OFFLINE_DELIVERIES, MAX_OFFLINE_BYTES);
    Ok(())
}

fn deliveries() -> &'static Mutex<OfflineStore> {
    OFFLINE_DELIVERIES.get_or_init(|| Mutex::new(OfflineStore::default()))
}

fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn delivery(subject: &str, attachment_bytes: usize) -> OfflineMockDelivery {
        OfflineMockDelivery {
            provider: "fixture".to_string(),
            delivery_id: subject.to_string(),
            message: Message::new().subject(subject).attach_bytes(
                "a.bin",
                vec![0_u8; attachment_bytes],
                "application/octet-stream",
            ),
        }
    }

    #[test]
    fn offline_store_keeps_only_the_newest_bounded_deliveries() {
        let mut store = OfflineStore::default();
        for index in 0..5 {
            store.push(delivery(&format!("m{index}"), 1), 3, usize::MAX);
        }
        let subjects: Vec<_> = store
            .deliveries
            .iter()
            .map(|(_, item)| item.message.subject.as_str())
            .collect();
        assert_eq!(subjects, ["m2", "m3", "m4"]);

        let mut store = OfflineStore::default();
        store.push(delivery("a", 40), 10, 100);
        store.push(delivery("b", 40), 10, 100);
        store.push(delivery("c", 40), 10, 100);
        assert_eq!(store.deliveries.len(), 2);
        assert!(store.bytes <= 100);
        // A single oversized delivery is still retained on its own.
        store.push(delivery("huge", 500), 10, 100);
        assert_eq!(store.deliveries.len(), 1);
        assert_eq!(store.bytes, 504);
    }

    #[test]
    fn credential_modes_are_explicit() {
        assert_eq!(credential_mode(""), DeliveryMode::OfflineMock);
        assert_eq!(credential_mode("mock_resend"), DeliveryMode::OfflineMock);
        assert_eq!(credential_mode("MoCk_provider"), DeliveryMode::OfflineMock);
        assert_eq!(credential_mode("real-secret"), DeliveryMode::Real);
    }
}
