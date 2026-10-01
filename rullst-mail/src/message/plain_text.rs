//! Plain-text fallback derived from an HTML body.

/// Converts HTML content into a clean, accessible plain-text representation for email clients.
///
/// The target of an absolute HTTP(S) link follows its label as `label <URL>`
/// (RFC 3986 Appendix C delimiters), unless the label already is that URL,
/// so text-only readers keep call-to-action links.
pub fn strip_html_to_plain_text(html: &str) -> String {
    if html.is_empty() {
        return String::new();
    }

    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut tag_buffer = String::new();
    let mut in_style_or_script = false;
    // The raw `href` of the open `<a>` and where its label starts in `result`.
    let mut open_link: Option<(String, usize)> = None;

    for c in html.chars() {
        if c == '<' {
            in_tag = true;
            tag_buffer.clear();
            continue;
        }

        if in_tag {
            if c == '>' {
                in_tag = false;
                let tag_lower = tag_buffer.trim().to_lowercase();

                let anchor =
                    !in_style_or_script && tag_lower.split_ascii_whitespace().next() == Some("a");
                if anchor {
                    open_link =
                        anchor_href(&tag_buffer).map(|href| (href.to_string(), result.len()));
                } else if tag_lower == "/a" {
                    if let Some((href, label_start)) = open_link.take() {
                        append_link_target(&mut result, label_start, &href);
                    }
                } else if tag_lower.starts_with("style") || tag_lower.starts_with("script") {
                    in_style_or_script = true;
                } else if tag_lower.starts_with("/style") || tag_lower.starts_with("/script") {
                    in_style_or_script = false;
                } else if tag_lower == "br"
                    || tag_lower == "br/"
                    || tag_lower == "br /"
                    || tag_lower == "p"
                    || tag_lower == "/p"
                    || tag_lower == "div"
                    || tag_lower == "/div"
                    || tag_lower == "tr"
                    || tag_lower == "/tr"
                    || tag_lower.starts_with("h1")
                    || tag_lower.starts_with("h2")
                    || tag_lower.starts_with("h3")
                    || tag_lower.starts_with("h4")
                    || tag_lower.starts_with("h5")
                    || tag_lower.starts_with("h6")
                    || tag_lower == "/h1"
                    || tag_lower == "/h2"
                    || tag_lower == "/h3"
                    || tag_lower == "/h4"
                    || tag_lower == "/h5"
                    || tag_lower == "/h6"
                {
                    result.push('\n');
                } else if tag_lower == "li" {
                    result.push_str("\n• ");
                }
            } else {
                tag_buffer.push(c);
            }
            continue;
        }

        if !in_style_or_script {
            result.push(c);
        }
    }

    // Decode numeric and common named references in one pass, so each is
    // decoded once (`&amp;lt;` stays `&lt;`).
    let decoded = crate::entities::decode(&result);

    // Normalize multiple newlines and spaces
    let mut normalized = String::new();
    let mut consecutive_newlines = 0;

    for line in decoded.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if consecutive_newlines < 1 {
                normalized.push('\n');
                consecutive_newlines += 1;
            }
        } else {
            if !normalized.is_empty() && !normalized.ends_with('\n') {
                normalized.push('\n');
            }
            normalized.push_str(trimmed);
            normalized.push('\n');
            consecutive_newlines = 0;
        }
    }

    normalized.trim().to_string()
}

/// The `href` value inside an `<a ...>` tag's text (between `<` and `>`). The
/// name is matched ASCII case-insensitively after whitespace, so `data-href`
/// does not count, and the value may be quoted or unquoted.
fn anchor_href(tag: &str) -> Option<&str> {
    let lower = tag.to_ascii_lowercase();
    let mut search = 0;
    while let Some(found) = lower.get(search..).and_then(|rest| rest.find("href")) {
        let at = search + found;
        search = at + "href".len();
        if !lower[..at].ends_with(|c: char| c.is_ascii_whitespace()) {
            continue;
        }
        let Some(value) = tag[search..].trim_start().strip_prefix('=') else {
            continue;
        };
        let value = value.trim_start();
        return match value.chars().next()? {
            quote @ ('"' | '\'') => {
                let inner = &value[1..];
                inner.find(quote).map(|end| &inner[..end])
            }
            _ => Some(
                &value[..value
                    .find(|c: char| c.is_ascii_whitespace())
                    .unwrap_or(value.len())],
            ),
        };
    }
    None
}

/// Appends ` <URL>` after a link label for an absolute HTTP(S) target that
/// is not already the label. `href` stays undecoded, as the whole result is
/// decoded once afterwards.
fn append_link_target(result: &mut String, label_start: usize, href: &str) {
    let target = crate::entities::decode(href);
    let target = target.trim();
    let scheme = target.get(..8).unwrap_or(target).to_ascii_lowercase();
    if !(scheme.starts_with("http://") || scheme.starts_with("https://"))
        || target.contains(|c: char| c.is_whitespace() || c.is_control())
    {
        return;
    }
    let label = result.get(label_start..).unwrap_or_default();
    if crate::entities::decode(label).trim() == target {
        return;
    }
    if !result.is_empty() && !result.ends_with(char::is_whitespace) {
        result.push(' ');
    }
    result.push('<');
    result.push_str(href.trim());
    result.push('>');
}
