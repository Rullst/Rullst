use super::{current_timestamp_str, normalize_ip};
use serde::{Deserialize, Serialize};

/// Frozen JSON schema version for [`LiveSecurityEvent`].
pub const SECURITY_EVENT_SCHEMA_VERSION: u16 = 1;

const MAX_EVENT_TYPE_BYTES: usize = 64;
const MAX_EVENT_DETAILS_BYTES: usize = 2 * 1024;

/// Bounded security event rendered by Studio/Nexus and accepted by local sinks.
///
/// Version 1 fields are serialized as `schema_version`, `event_type`, `details`,
/// `client_ip`, `timestamp_str`, and `verified_hmac`. The boolean reports only
/// cryptographic event integrity; it does not mean the event source or claim is
/// trusted. Locally constructed events are always unsigned. `rullst-security`
/// publishes the JSON Schema of version 1 as
/// `rullst_security::LIVE_SECURITY_EVENT_V1_JSON_SCHEMA`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LiveSecurityEvent {
    /// Schema version. Missing values in legacy JSON deserialize as version 1.
    #[serde(default = "security_event_schema_version")]
    pub schema_version: u16,
    /// Uppercase ASCII identifier, at most 64 bytes.
    pub event_type: String,
    /// Human-readable, unstructured detail text, at most 2 KiB.
    pub details: String,
    /// Canonical IP address or `unknown`.
    pub client_ip: String,
    /// Absolute RFC 3339 timestamp.
    pub timestamp_str: String,
    /// Whether a connected verifier validated an HMAC for this exact event.
    pub verified_hmac: bool,
}

impl LiveSecurityEvent {
    /// Creates an unsigned local event and normalizes every untrusted field.
    pub fn local(
        event_type: impl Into<String>,
        details: impl Into<String>,
        client_ip: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: SECURITY_EVENT_SCHEMA_VERSION,
            event_type: event_type.into(),
            details: details.into(),
            client_ip: client_ip.into(),
            timestamp_str: current_timestamp_str(),
            verified_hmac: false,
        }
        .normalized()
    }

    /// The event with the version 1 bounds applied: the current schema
    /// version, `SECURITY_EVENT` for an invalid type, details cut to 2 KiB on
    /// a character boundary, a canonical IP or `unknown`, and the current time
    /// for a timestamp that is not RFC 3339.
    // Used by the SIEM spools and the CEF formatter of `rullst-security`.
    #[doc(hidden)]
    pub fn normalized(mut self) -> Self {
        self.schema_version = SECURITY_EVENT_SCHEMA_VERSION;
        if !valid_event_type(&self.event_type) {
            self.event_type = "SECURITY_EVENT".to_string();
        }
        truncate_utf8(&mut self.details, MAX_EVENT_DETAILS_BYTES);
        self.client_ip = normalize_ip(&self.client_ip);
        if chrono::DateTime::parse_from_rfc3339(&self.timestamp_str).is_err() {
            self.timestamp_str = current_timestamp_str();
        }
        self
    }
}

const fn security_event_schema_version() -> u16 {
    SECURITY_EVENT_SCHEMA_VERSION
}

fn valid_event_type(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_EVENT_TYPE_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}

fn truncate_utf8(value: &mut String, max_bytes: usize) {
    if value.len() <= max_bytes {
        return;
    }
    let mut boundary = max_bytes;
    while boundary > 0 && !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    value.truncate(boundary);
}

#[cfg(test)]
#[path = "event_tests.rs"]
mod tests;
