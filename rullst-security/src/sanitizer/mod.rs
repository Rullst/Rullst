pub mod csp;

use ammonia::clean;

pub struct HtmlSanitizer;

impl HtmlSanitizer {
    pub fn sanitize(dirty_html: &str) -> String {
        clean(dirty_html)
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
