//! Anchor `href` rewriting for click tracking.

use super::TrackingError;

/// Replaces the double-quoted absolute HTTP(S) `href` of each `<a>` element
/// with `{tracker_base}/track/click/{token}`.
///
/// Other elements keep their `href`: a `<link>` stylesheet or `<base>` that
/// a mail client fetches on open must not register as a click or consume a
/// one-time click token. A destination that fails the pipeline's link or
/// secret checks is left in place for that pipeline to reject or redact. `tracker_base` must already be escaped for an
/// attribute value. The token signs the destination as a browser reads it,
/// with character references decoded, so `?a=1&amp;b=2` is signed and later
/// redirected as `?a=1&b=2`.
pub(super) fn rewrite_links(
    html: &str,
    tracker_base: &str,
    mut sign: impl FnMut(&str) -> Result<String, TrackingError>,
) -> Result<String, TrackingError> {
    const PATTERN: &str = "href=\"";
    let mut output = String::with_capacity(html.len() + 256);
    let mut last_index = 0;
    let mut in_anchor = false;
    while let Some(relative) = html[last_index..].find(PATTERN) {
        let attribute = last_index + relative;
        let href_start = attribute + PATTERN.len();
        output.push_str(&html[last_index..href_start]);
        in_anchor = inside_anchor_tag(&html[last_index..attribute], in_anchor);
        let Some(end) = html[href_start..].find('"') else {
            last_index = href_start;
            break;
        };
        let url_end = href_start + end;
        let raw = &html[href_start..url_end];
        let target = crate::entities::decode(raw);
        // `data-href` and other names merely ending in `href` are not links.
        let is_href = html[..attribute]
            .bytes()
            .next_back()
            .is_some_and(|byte| byte.is_ascii_whitespace());
        if in_anchor
            && is_href
            && (target.starts_with("http://") || target.starts_with("https://"))
            && passes_pipeline_checks(&target)
        {
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

/// Whether the mandatory pipeline would accept `target` unchanged. That
/// pipeline later sees only the tracker URL, so a destination it would reject
/// (homograph host) or redact (credentials) keeps its original `href`, where
/// the pipeline then rejects or redacts it as it does for untracked mail.
fn passes_pipeline_checks(target: &str) -> bool {
    crate::security::scan_content_security(target).is_ok()
        && crate::action::redact_body_secrets(target) == target
}

/// Whether an attribute that follows `gap` still sits inside an `<a>` start
/// tag. `gap` is the text since the previous attribute value, so each byte
/// is examined once and `previous` carries the state across attributes of
/// one tag. A `>` inside an earlier quoted value leaves that link untracked.
fn inside_anchor_tag(gap: &str, previous: bool) -> bool {
    match gap.rfind('<') {
        Some(open) => {
            let tag = &gap[open + 1..];
            let name_end = tag
                .find(|c: char| c.is_ascii_whitespace() || matches!(c, '/' | '>'))
                .unwrap_or(tag.len());
            tag[..name_end].eq_ignore_ascii_case("a") && !tag.contains('>')
        }
        None => previous && !gap.contains('>'),
    }
}
