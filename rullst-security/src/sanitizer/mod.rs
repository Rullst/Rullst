//! HTML sanitization and a minimal CSP layer.
//!
//! **Risk reduced:** stored or reflected XSS when you must render HTML that a
//! user supplied (comments, rich-text fields).
//!
//! **How:** an allowlist. [`HtmlSanitizer::sanitize`] uses `ammonia`'s default
//! policy, which keeps a fixed set of formatting tags and safe attributes and
//! removes scripts, event handlers, unsafe URL schemes and unknown elements.
//! [`HtmlSanitizer::sanitize_text`] escapes `&`, `<`, `>`, `"` and `'`.
//!
//! **Known limits:** sanitized HTML can still contain links and images to
//! external sites, so content can still track readers or mislead them.
//! `sanitize_text` is correct only for HTML text and quoted attribute values,
//! not inside `<script>`, `<style>`, URLs or unquoted attributes.
//! [`csp::CspSecurityLayer`] always sends the default CSP.
//!
//! **Operator duties:** prefer the escaping `html!` macro over raw HTML,
//! sanitize on output for the context you render into, and keep a strict CSP.

pub mod csp;

use ammonia::clean;

/// HTML sanitizer and text escaper; see the module documentation.
pub struct HtmlSanitizer;

impl HtmlSanitizer {
    /// Removes everything outside `ammonia`'s default allowlist and records a
    /// telemetry event when the input changed.
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

    /// Escapes text for an HTML text node or a quoted attribute value.
    pub fn sanitize_text(input: &str) -> String {
        input
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
    }
}
