use super::{current_timestamp_str, normalize_ip};
use serde::{Deserialize, Serialize};

/// Frozen JSON schema version for [`LiveSecurityEvent`].
pub const SECURITY_EVENT_SCHEMA_VERSION: u16 = 1;

/// Machine-readable JSON Schema 2020-12 contract for [`LiveSecurityEvent`] v1.
pub const LIVE_SECURITY_EVENT_V1_JSON_SCHEMA: &str =
    include_str!("../../schema/security-event-v1.schema.json");

const MAX_EVENT_TYPE_BYTES: usize = 64;
const MAX_EVENT_DETAILS_BYTES: usize = 2 * 1024;

/// Bounded security event rendered by Studio/Nexus and accepted by local sinks.
///
/// Version 1 fields are serialized as `schema_version`, `event_type`, `details`,
/// `client_ip`, `timestamp_str`, and `verified_hmac`. The boolean reports only
/// cryptographic event integrity; it does not mean the event source or claim is
/// trusted. Locally constructed events are always unsigned.
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

    pub(crate) fn normalized(mut self) -> Self {
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
mod tests {
    use super::*;

    #[test]
    fn version_one_json_contract_is_exact_and_legacy_input_defaults_to_v1() {
        let event = LiveSecurityEvent::local("RBAC_DENIAL", "denied", "192.0.2.4");
        let value = serde_json::to_value(&event).expect("serialize event");
        let object = value.as_object().expect("event object");
        let mut keys = object.keys().map(String::as_str).collect::<Vec<_>>();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "client_ip",
                "details",
                "event_type",
                "schema_version",
                "timestamp_str",
                "verified_hmac"
            ]
        );
        assert_eq!(value["schema_version"], SECURITY_EVENT_SCHEMA_VERSION);

        let legacy = serde_json::json!({
            "event_type": "RBAC_DENIAL",
            "details": "denied",
            "client_ip": "192.0.2.4",
            "timestamp_str": "2026-08-27T12:00:00Z",
            "verified_hmac": false
        });
        let decoded: LiveSecurityEvent =
            serde_json::from_value(legacy).expect("legacy event remains readable");
        assert_eq!(decoded.schema_version, SECURITY_EVENT_SCHEMA_VERSION);
    }

    #[test]
    fn bundled_json_schema_declares_the_exact_version_one_field_set() {
        let schema: serde_json::Value =
            serde_json::from_str(LIVE_SECURITY_EVENT_V1_JSON_SCHEMA).expect("valid JSON Schema");
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(schema["properties"]["schema_version"]["const"], 1);
        let required = schema["required"]
            .as_array()
            .expect("required array")
            .iter()
            .filter_map(serde_json::Value::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        let serialized = serde_json::to_value(LiveSecurityEvent::local(
            "SECURITY_EVENT",
            "details",
            "unknown",
        ))
        .expect("serialize event");
        let fields = serialized
            .as_object()
            .expect("event object")
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(required, fields);
    }

    #[test]
    fn local_events_normalize_identity_type_timestamp_and_utf8_size() {
        let oversized = "á".repeat(MAX_EVENT_DETAILS_BYTES);
        let mut event = LiveSecurityEvent {
            schema_version: 900,
            event_type: "invalid type".to_string(),
            details: oversized,
            client_ip: "attacker-controlled".to_string(),
            timestamp_str: "moments ago".to_string(),
            verified_hmac: false,
        }
        .normalized();

        assert_eq!(event.schema_version, SECURITY_EVENT_SCHEMA_VERSION);
        assert_eq!(event.event_type, "SECURITY_EVENT");
        assert_eq!(event.client_ip, "unknown");
        assert!(event.details.len() <= MAX_EVENT_DETAILS_BYTES);
        assert!(event.details.is_char_boundary(event.details.len()));
        assert!(chrono::DateTime::parse_from_rfc3339(&event.timestamp_str).is_ok());

        event.event_type = "A".repeat(MAX_EVENT_TYPE_BYTES + 1);
        assert_eq!(event.normalized().event_type, "SECURITY_EVENT");
    }
}
