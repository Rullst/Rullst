use crate::telemetry::{LiveSecurityEvent, SecurityStore};
use serde::{Deserialize, Serialize};

mod authenticated;
mod spool;
pub use authenticated::{
    AuthenticatedSiemSpool, AuthenticatedSiemSpoolError, AuthenticatedSiemSpoolReceipt,
    AuthenticatedSiemSpoolSnapshot, MAX_SIEM_INTEGRITY_KEYS, SiemIntegrityKey, SiemKeyRing,
};
pub use spool::{
    DurableSiemSpool, MAX_SIEM_SPOOL_BYTES, MAX_SIEM_SPOOL_RECORDS, SiemSpoolError,
    SiemSpoolReceipt, SiemSpoolSnapshot,
};

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
        event.event_type,
        event.event_type,
        severity,
        escape_cef_extension(&event.client_ip),
        escape_cef_extension(&event.details)
    )
}

fn escape_cef_extension(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '=' => escaped.push_str("\\="),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            _ => escaped.push(character),
        }
    }
    escaped
}

/// Records a SIEM-candidate alert in the bounded local telemetry store.
///
/// Despite the compatibility name, this function does not deliver events to an
/// external SIEM. Use [`format_cef_event`] to serialize a local event; durable
/// transport, retry, dead-letter handling and delivery acknowledgement remain
/// application-owned until a real sink contract is implemented.
pub fn dispatch_siem_alert(event_type: &str, details: &str, client_ip: &str) {
    let event = LiveSecurityEvent::local(event_type, details, client_ip);

    SecurityStore::global().inc_siem_dispatches();

    SecurityStore::global().push_local_event(event);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_cef_event() {
        let ev = LiveSecurityEvent::local(
            "HONEYPOT_TRAP_TRIGGERED",
            "IP 10.0.0.1 accessed /.env",
            "10.0.0.1",
        );

        let cef = format_cef_event(&ev);
        assert!(cef.starts_with("CEF:0|RullstSecurity|Framework|12.0.0|HONEYPOT_TRAP_TRIGGERED"));
        assert!(cef.contains("src=10.0.0.1"));
    }

    #[test]
    fn cef_extension_values_cannot_inject_fields_or_lines() {
        let event = LiveSecurityEvent::local(
            "SECURITY_EVENT",
            "first=value\\second\nnext=field\r",
            "192.0.2.8",
        );
        let cef = format_cef_event(&event);
        assert!(cef.contains(r"msg=first\=value\\second\nnext\=field\r"));
        assert!(!cef.contains('\n'));
        assert!(!cef.contains('\r'));
    }
}
