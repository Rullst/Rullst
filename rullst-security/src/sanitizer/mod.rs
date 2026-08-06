pub mod csp;

use ammonia::clean;

pub struct HtmlSanitizer;

impl HtmlSanitizer {
    pub fn sanitize(dirty_html: &str) -> String {
        let cleaned = clean(dirty_html);
        if cleaned != dirty_html {
            let store = crate::telemetry::SecurityStore::global();
            store.inc_sanitizations();
            if let Ok(mut events) = store.live_events.lock() {
                events.insert(
                    0,
                    crate::telemetry::LiveSecurityEvent {
                        event_type: "XSS_SANITIZED".to_string(),
                        details: "Sanitized unsafe HTML/SVG tags or attributes".to_string(),
                        client_ip: "127.0.0.1".to_string(),
                        timestamp_str: crate::telemetry::current_timestamp_str(),
                        verified_hmac: true,
                    },
                );
            }
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
