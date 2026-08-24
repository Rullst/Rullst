//! Deterministic, process-local fallback used by providers with mock credentials.

use crate::error::MailError;
use crate::message::Message;
use sha2::{Digest, Sha256};
use std::sync::{Mutex, OnceLock};

static OFFLINE_DELIVERIES: OnceLock<Mutex<Vec<OfflineMockDelivery>>> = OnceLock::new();

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
        deliveries()
            .lock()
            .map_err(|_| MailError::DriverError("offline mock store lock poisoned".to_string()))?
            .clear();
        Ok(())
    }

    /// Returns a snapshot of all captured offline deliveries.
    pub fn deliveries() -> Result<Vec<OfflineMockDelivery>, MailError> {
        deliveries()
            .lock()
            .map(|items| items.clone())
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
    deliveries()
        .lock()
        .map_err(|_| MailError::DriverError("offline mock store lock poisoned".to_string()))?
        .push(delivery);
    Ok(())
}

fn deliveries() -> &'static Mutex<Vec<OfflineMockDelivery>> {
    OFFLINE_DELIVERIES.get_or_init(|| Mutex::new(Vec::new()))
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

    #[test]
    fn credential_modes_are_explicit() {
        assert_eq!(credential_mode(""), DeliveryMode::OfflineMock);
        assert_eq!(credential_mode("mock_resend"), DeliveryMode::OfflineMock);
        assert_eq!(credential_mode("MoCk_provider"), DeliveryMode::OfflineMock);
        assert_eq!(credential_mode("real-secret"), DeliveryMode::Real);
    }
}
