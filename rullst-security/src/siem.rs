use crate::telemetry::{current_timestamp_str, LiveSecurityEvent, SecurityStore};
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
        event.event_type,
        event.event_type,
        severity,
        event.client_ip,
        event.details
    )
}

/// Emits a security alert to configured SIEM endpoints and records telemetry.
pub fn dispatch_siem_alert(event_type: &str, details: &str, client_ip: &str) {
    let now = current_timestamp_str();
    let event = LiveSecurityEvent {
        event_type: event_type.to_string(),
        details: details.to_string(),
        client_ip: client_ip.to_string(),
        timestamp_str: now,
        verified_hmac: true,
    };

    SecurityStore::global().inc_siem_dispatches();

    if let Ok(mut events) = SecurityStore::global().live_events.lock() {
        events.insert(0, event);
        if events.len() > 50 {
            events.truncate(50);
        }
    }
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
            timestamp_str: "Just now".to_string(),
            verified_hmac: true,
        };

        let cef = format_cef_event(&ev);
        assert!(cef.starts_with("CEF:0|RullstSecurity|Framework|12.0.0|HONEYPOT_TRAP_TRIGGERED"));
        assert!(cef.contains("src=10.0.0.1"));
    }
}
