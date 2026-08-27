use crate::telemetry::{LiveSecurityEvent, SecurityStore, current_timestamp_str, normalize_ip};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SiemAlertPayload {
    pub version: String,
    pub event_type: String,
    pub severity: String,
    pub details: String,
    pub client_ip: String,
    pub timestamp_str: String,
}

/// Formats a LiveSecurityEvent into Common Event Format (CEF) string.
pub fn format_cef_event(event: &LiveSecurityEvent) -> String {
    let severity = match event.event_type.as_str() {
        "HONEYPOT_TRAP_TRIGGERED" => "8",
        "AI_PROMPT_INJECTION_SHIELDED" => "9",
        "XSS_PAYLOAD_NEUTRALIZED" => "7",
        _ => "5",
    };

    format!(
        "CEF:0|RullstSecurity|Framework|12.0.0|{}|{}|{}|src={} msg={}",
        event.event_type, event.event_type, severity, event.client_ip, event.details
    )
}

/// Records a SIEM-candidate alert in the bounded local telemetry store.
///
/// Despite the compatibility name, this function does not deliver events to an
/// external SIEM. Use [`format_cef_event`] to serialize a local event; durable
/// transport, retry, dead-letter handling and delivery acknowledgement remain
/// application-owned until a real sink contract is implemented.
pub fn dispatch_siem_alert(event_type: &str, details: &str, client_ip: &str) {
    let now = current_timestamp_str();
    let event = LiveSecurityEvent {
        event_type: event_type.to_string(),
        details: details.to_string(),
        client_ip: normalize_ip(client_ip),
        timestamp_str: now,
        verified_hmac: false,
    };

    SecurityStore::global().inc_siem_dispatches();

    SecurityStore::global().push_local_event(event);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_cef_event() {
        let ev = LiveSecurityEvent {
            event_type: "HONEYPOT_TRAP_TRIGGERED".to_string(),
            details: "IP 10.0.0.1 accessed /.env".to_string(),
            client_ip: "10.0.0.1".to_string(),
            timestamp_str: "2026-08-20T12:00:00.000Z".to_string(),
            verified_hmac: false,
        };

        let cef = format_cef_event(&ev);
        assert!(cef.starts_with("CEF:0|RullstSecurity|Framework|12.0.0|HONEYPOT_TRAP_TRIGGERED"));
        assert!(cef.contains("src=10.0.0.1"));
    }
}
