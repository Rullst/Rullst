pub mod csp;

use ammonia::clean;

pub struct HtmlSanitizer;

impl HtmlSanitizer {
    pub fn sanitize(dirty_html: &str) -> String {
        let cleaned = clean(dirty_html);
        if cleaned != dirty_html {
            let store = crate::telemetry::SecurityStore::global();
            store.inc_sanitizations();
            store.push_local_event(crate::telemetry::LiveSecurityEvent {
                event_type: "XSS_SANITIZED".to_string(),
                details: "Sanitized unsafe HTML/SVG tags or attributes".to_string(),
                client_ip: "unknown".to_string(),
                timestamp_str: crate::telemetry::current_timestamp_str(),
                verified_hmac: false,
            });
        }
        cleaned
    }

    pub fn sanitize_text(input: &str) -> String {
        input
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
    }
}
