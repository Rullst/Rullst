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
///
/// The event is normalized first, exactly as the framework's own sinks do, so
/// an event built or deserialized outside that path cannot place `|`, `\` or
/// a line break in the CEF header: an event type that is not an uppercase
/// ASCII identifier of at most 64 bytes is exported as `SECURITY_EVENT`.
pub fn format_cef_event(event: &LiveSecurityEvent) -> String {
    let event = event.clone().normalized();
    let severity = match event.event_type.as_str() {
        "HONEYPOT_TRAP_TRIGGERED" => "8",
        "AI_PROMPT_INJECTION_SHIELDED" => "9",
        // `HtmlSanitizer` reports XSS_SANITIZED; the telemetry helper reports
        // XSS_PAYLOAD_NEUTRALIZED. Both are the same neutralized XSS payload.
        "XSS_PAYLOAD_NEUTRALIZED" | "XSS_SANITIZED" => "7",
        _ => "5",
    };

    format!(
        "CEF:0|RullstSecurity|Framework|{}|{}|{}|{}|src={} msg={}",
        env!("CARGO_PKG_VERSION"),
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
        assert!(cef.starts_with(&format!(
            "CEF:0|RullstSecurity|Framework|{}|HONEYPOT_TRAP_TRIGGERED",
            env!("CARGO_PKG_VERSION")
        )));
        assert!(cef.contains("src=10.0.0.1"));
    }

    #[test]
    fn both_xss_event_names_export_the_xss_severity() {
        crate::sanitizer::HtmlSanitizer::sanitize("<script>alert(1)</script><p>x</p>");
        let sanitized = crate::telemetry::SecurityStore::global()
            .live_events
            .lock()
            .unwrap()
            .iter()
            .find(|event| event.event_type == "XSS_SANITIZED")
            .cloned()
            .expect("the sanitizer records an event");
        for event in [
            sanitized,
            LiveSecurityEvent::local("XSS_PAYLOAD_NEUTRALIZED", "x", "unknown"),
        ] {
            let cef = format_cef_event(&event);
            assert!(
                cef.contains(&format!("|{0}|{0}|7|", event.event_type)),
                "{cef}"
            );
        }
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

    #[test]
    fn unnormalized_event_types_cannot_forge_cef_header_fields() {
        let event = LiveSecurityEvent {
            schema_version: 1,
            event_type:
                "LOGIN_OK|LOGIN_OK|0|src=10.0.0.1 msg=ok\nCEF:0|RullstSecurity|Framework|1|X|X|10|"
                    .to_string(),
            details: "ok".to_string(),
            client_ip: "10.0.0.1".to_string(),
            timestamp_str: "2026-09-30T00:00:00Z".to_string(),
            verified_hmac: false,
        };
        let cef = format_cef_event(&event);
        assert!(!cef.contains('\n') && !cef.contains('\r'));
        assert_eq!(
            cef,
            format!(
                "CEF:0|RullstSecurity|Framework|{}|SECURITY_EVENT|SECURITY_EVENT|5|src=10.0.0.1 msg=ok",
                env!("CARGO_PKG_VERSION")
            )
        );
    }
}
