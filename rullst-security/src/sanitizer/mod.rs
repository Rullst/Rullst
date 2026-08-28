pub mod csp;

use ammonia::clean;

pub struct HtmlSanitizer;

impl HtmlSanitizer {
    pub fn sanitize(dirty_html: &str) -> String {
        let cleaned = clean(dirty_html);
        if cleaned != dirty_html {
            let store = crate::telemetry::SecurityStore::global();
            store.inc_sanitizations();
            store.push_local_event(crate::telemetry::LiveSecurityEvent::local(
                "XSS_SANITIZED",
                "Sanitized unsafe HTML/SVG tags or attributes",
                "unknown",
            ));
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
