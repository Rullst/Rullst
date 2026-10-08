//! Whitespace normalisation and volatile-value masking for HTML snapshots.
//!
//! The normaliser is deliberately small and dependency-free: it is not an
//! HTML parser, only a tokenizer that knows tags, comments and the raw-text
//! elements whose content must stay byte-for-byte (`pre`, `textarea`,
//! `script` and `style`).

use super::snapshot::SnapshotOptions;

/// Written in place of a CSP nonce when [`SnapshotOptions::mask_nonce`] is set.
pub const NONCE_PLACEHOLDER: &str = "{NONCE}";
/// Written in place of a CSRF token when [`SnapshotOptions::mask_csrf_token`] is set.
pub const CSRF_TOKEN_PLACEHOLDER: &str = "{CSRF_TOKEN}";

/// Elements whose content is copied verbatim.
const RAW_TEXT_ELEMENTS: [&str; 4] = ["pre", "textarea", "script", "style"];
/// The spellings of a double or single quote inside an attribute value.
const QUOTES: [&str; 5] = ["&quot;", "&#34;", "&#x22;", "\"", "'"];

/// Normalises rendered HTML so a snapshot only changes when the markup does.
///
/// - Line endings become `\n`.
/// - Whitespace-only text between two tags is dropped and every tag that
///   directly follows another tag starts a new line, so a one-line `html!`
///   result diffs line by line.
/// - Other runs of ASCII whitespace in text and inside tags collapse to one
///   space; quoted attribute values, comments and the content of `pre`,
///   `textarea`, `script` and `style` are kept as written.
/// - With [`SnapshotOptions`], CSP nonces become [`NONCE_PLACEHOLDER`] and
///   CSRF tokens become [`CSRF_TOKEN_PLACEHOLDER`].
///
/// Normalising twice gives the same result as normalising once.
///
/// ```
/// use rullst_core::testing::{SnapshotOptions, normalize_html};
///
/// let page = "<ul>\n  <li>One</li>   <li>Two</li>\n</ul>\
///             <script nonce=\"per-response\" src=\"/app.js\"></script>";
/// assert_eq!(
///     normalize_html(page, SnapshotOptions::new().mask_nonce()),
///     "<ul>\n<li>One</li>\n<li>Two</li>\n</ul>\n<script nonce=\"{NONCE}\" src=\"/app.js\"></script>\n"
/// );
/// ```
pub fn normalize_html(html: &str, options: SnapshotOptions) -> String {
    let source = html.replace("\r\n", "\n").replace('\r', "\n");
    let mut out = String::with_capacity(source.len() + 16);
    let mut after_tag = false;
    let mut rest = source.as_str();
    while !rest.is_empty() {
        if rest.starts_with("<!--") {
            let end = rest[4..].find("-->").map_or(rest.len(), |index| index + 7);
            push_tag(&mut out, &rest[..end], &mut after_tag);
            rest = &rest[end..];
        } else if starts_tag(rest) {
            let end = tag_end(rest);
            let raw_tag = &rest[..end];
            push_tag(&mut out, &normalize_tag(raw_tag, options), &mut after_tag);
            rest = &rest[end..];
            if let Some(name) = raw_text_element(raw_tag) {
                let close = closing_tag(rest, name);
                out.push_str(&rest[..close]);
                after_tag = false;
                rest = &rest[close..];
            }
        } else {
            let end = next_tag(rest);
            let text = &rest[..end];
            if !text.bytes().all(|byte| byte.is_ascii_whitespace()) {
                out.push_str(&collapse_whitespace(text));
                after_tag = false;
            }
            rest = &rest[end..];
        }
    }
    let mut normalized = out
        .trim_matches(|c: char| c.is_ascii_whitespace())
        .to_string();
    normalized.push('\n');
    normalized
}

fn push_tag(out: &mut String, tag: &str, after_tag: &mut bool) {
    if *after_tag {
        out.push('\n');
    }
    out.push_str(tag);
    *after_tag = true;
}

/// `<` followed by a letter, `/`, `!` or `?` opens a tag; `a < b` is text.
fn starts_tag(text: &str) -> bool {
    let mut bytes = text.bytes();
    bytes.next() == Some(b'<')
        && bytes
            .next()
            .is_some_and(|byte| byte.is_ascii_alphabetic() || matches!(byte, b'/' | b'!' | b'?'))
}

/// The byte index just after the tag's closing `>`, ignoring quoted `>`.
fn tag_end(text: &str) -> usize {
    let mut quote = None;
    for (index, byte) in text.bytes().enumerate().skip(1) {
        match quote {
            Some(open) if byte == open => quote = None,
            Some(_) => {}
            None if byte == b'"' || byte == b'\'' => quote = Some(byte),
            None if byte == b'>' => return index + 1,
            None => {}
        }
    }
    text.len()
}

/// The start of the next tag or comment after the first byte.
fn next_tag(text: &str) -> usize {
    text.char_indices()
        .skip(1)
        .find(|(index, character)| *character == '<' && starts_tag(&text[*index..]))
        .map_or(text.len(), |(index, _)| index)
}

/// The lowercase tag name, without a leading `/`.
fn tag_name(tag: &str) -> String {
    tag.trim_start_matches('<')
        .trim_start_matches('/')
        .bytes()
        .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'-')
        .map(|byte| char::from(byte.to_ascii_lowercase()))
        .collect()
}

fn raw_text_element(tag: &str) -> Option<&'static str> {
    if tag.starts_with("</") || tag.ends_with("/>") {
        return None;
    }
    let name = tag_name(tag);
    RAW_TEXT_ELEMENTS
        .into_iter()
        .find(|element| *element == name)
}

/// The index of `</name` (any letter case) that closes a raw-text element.
fn closing_tag(text: &str, name: &str) -> usize {
    let lower = text.to_ascii_lowercase();
    let needle = format!("</{name}");
    let mut from = 0;
    while let Some(found) = lower[from..].find(&needle) {
        let start = from + found;
        let after = lower.as_bytes().get(start + needle.len()).copied();
        if after.is_none_or(|byte| byte == b'>' || byte == b'/' || byte.is_ascii_whitespace()) {
            return start;
        }
        from = start + needle.len();
    }
    text.len()
}

fn collapse_whitespace(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_space = false;
    for character in text.chars() {
        if character.is_ascii_whitespace() {
            if !in_space {
                out.push(' ');
            }
            in_space = true;
        } else {
            out.push(character);
            in_space = false;
        }
    }
    out
}

/// Collapses whitespace outside quoted values, then masks volatile values.
fn normalize_tag(tag: &str, options: SnapshotOptions) -> String {
    let mut out = String::with_capacity(tag.len());
    let mut quote = None;
    let mut in_space = false;
    for character in tag.chars() {
        match quote {
            Some(open) => {
                out.push(character);
                if character == open {
                    quote = None;
                }
            }
            None if character.is_ascii_whitespace() => {
                if !in_space {
                    out.push(' ');
                }
                in_space = true;
                continue;
            }
            None => {
                if character == '"' || character == '\'' {
                    quote = Some(character);
                }
                if character == '>' && in_space {
                    out.pop();
                }
                out.push(character);
            }
        }
        in_space = false;
    }
    if options.masks_nonce() {
        out = mask_attribute(&out, "nonce", NONCE_PLACEHOLDER);
        out = mask_nonce_sources(&out);
    }
    if options.masks_csrf_token() {
        let name = attribute_value(&out, "name").map(str::to_ascii_lowercase);
        if name.as_deref() == Some("_token") {
            out = mask_attribute(&out, "value", CSRF_TOKEN_PLACEHOLDER);
        }
        if tag_name(&out) == "meta" && name.as_deref() == Some("csrf-token") {
            out = mask_attribute(&out, "content", CSRF_TOKEN_PLACEHOLDER);
        }
        out = mask_csrf_headers(&out);
    }
    out
}

/// One attribute: its name and the byte range of its value without quotes.
struct Attribute<'a> {
    name: &'a str,
    value: Option<std::ops::Range<usize>>,
}

fn attributes(tag: &str) -> Vec<Attribute<'_>> {
    let bytes = tag.as_bytes();
    let mut index = 1;
    while index < bytes.len() && !bytes[index].is_ascii_whitespace() && bytes[index] != b'>' {
        index += 1;
    }
    let mut found = Vec::new();
    while index < bytes.len() {
        while index < bytes.len() && (bytes[index].is_ascii_whitespace() || bytes[index] == b'/') {
            index += 1;
        }
        let start = index;
        while index < bytes.len()
            && !bytes[index].is_ascii_whitespace()
            && !matches!(bytes[index], b'=' | b'>' | b'/')
        {
            index += 1;
        }
        if start == index {
            break;
        }
        let name = &tag[start..index];
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if bytes.get(index) != Some(&b'=') {
            found.push(Attribute { name, value: None });
            continue;
        }
        index += 1;
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        let value = match bytes.get(index) {
            Some(&quote @ (b'"' | b'\'')) => {
                let begin = index + 1;
                let end = tag[begin..]
                    .bytes()
                    .position(|byte| byte == quote)
                    .map_or(tag.len(), |offset| begin + offset);
                index = (end + 1).min(tag.len());
                begin..end
            }
            _ => {
                let begin = index;
                while index < bytes.len()
                    && !bytes[index].is_ascii_whitespace()
                    && bytes[index] != b'>'
                {
                    index += 1;
                }
                begin..index
            }
        };
        found.push(Attribute {
            name,
            value: Some(value),
        });
    }
    found
}

fn attribute_value<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    attributes(tag)
        .into_iter()
        .find(|attribute| attribute.name.eq_ignore_ascii_case(name))
        .and_then(|attribute| attribute.value)
        .map(|range| &tag[range])
}

fn mask_attribute(tag: &str, name: &str, placeholder: &str) -> String {
    let range = attributes(tag)
        .into_iter()
        .find(|attribute| attribute.name.eq_ignore_ascii_case(name))
        .and_then(|attribute| attribute.value);
    match range {
        Some(range) if !range.is_empty() => {
            format!("{}{placeholder}{}", &tag[..range.start], &tag[range.end..])
        }
        _ => tag.to_string(),
    }
}

/// `'nonce-…'` CSP source expressions, for example in a CSP `<meta>`.
fn mask_nonce_sources(tag: &str) -> String {
    let mut out = String::with_capacity(tag.len());
    let mut rest = tag;
    while let Some(found) = rest.find("'nonce-") {
        let start = found + "'nonce-".len();
        let Some(length) = rest[start..].find('\'') else {
            break;
        };
        out.push_str(&rest[..start]);
        out.push_str(NONCE_PLACEHOLDER);
        rest = &rest[start + length..];
    }
    out.push_str(rest);
    out
}

fn quote_at(text: &str) -> Option<&'static str> {
    QUOTES.into_iter().find(|quote| text.starts_with(quote))
}

/// The value of an `X-CSRF-Token` key, as in `hx-headers='{"X-CSRF-Token": "…"}'`.
fn mask_csrf_headers(tag: &str) -> String {
    const KEY: &str = "x-csrf-token";
    let lower = tag.to_ascii_lowercase();
    let mut out = String::with_capacity(tag.len());
    let mut copied = 0;
    let mut from = 0;
    while let Some(found) = lower[from..].find(KEY) {
        let mut index = from + found + KEY.len();
        from = index;
        index += quote_at(&tag[index..]).map_or(0, str::len);
        index += tag[index..].len() - tag[index..].trim_start().len();
        if !tag[index..].starts_with(':') {
            continue;
        }
        index += 1;
        index += tag[index..].len() - tag[index..].trim_start().len();
        let Some(quote) = quote_at(&tag[index..]) else {
            continue;
        };
        let start = index + quote.len();
        let Some(length) = tag[start..].find(quote) else {
            continue;
        };
        out.push_str(&tag[copied..start]);
        out.push_str(CSRF_TOKEN_PLACEHOLDER);
        copied = start + length;
        from = copied;
    }
    out.push_str(&tag[copied..]);
    out
}
