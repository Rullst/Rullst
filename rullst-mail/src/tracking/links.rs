//! `href` rewriting for click tracking.

use super::TrackingError;

/// Replaces each double-quoted absolute HTTP(S) `href` value with
/// `{tracker_base}/track/click/{token}`.
///
/// `tracker_base` must already be escaped for an attribute value. The token
/// signs the destination as a browser reads it, with character references
/// decoded, so `?a=1&amp;b=2` is signed and later redirected as `?a=1&b=2`.
pub(super) fn rewrite_links(
    html: &str,
    tracker_base: &str,
    mut sign: impl FnMut(&str) -> Result<String, TrackingError>,
) -> Result<String, TrackingError> {
    const PATTERN: &str = "href=\"";
    let mut output = String::with_capacity(html.len() + 256);
    let mut last_index = 0;
    while let Some(relative) = html[last_index..].find(PATTERN) {
        let href_start = last_index + relative + PATTERN.len();
        output.push_str(&html[last_index..href_start]);
        let Some(end) = html[href_start..].find('"') else {
            last_index = href_start;
            break;
        };
        let url_end = href_start + end;
        let raw = &html[href_start..url_end];
        let target = crate::entities::decode(raw);
        if target.starts_with("http://") || target.starts_with("https://") {
            output.push_str(tracker_base);
            output.push_str("/track/click/");
            output.push_str(&sign(&target)?);
        } else {
            output.push_str(raw);
        }
        last_index = url_end;
    }
    output.push_str(&html[last_index..]);
    Ok(output)
}
