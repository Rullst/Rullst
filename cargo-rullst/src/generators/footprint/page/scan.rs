//! The resources a page makes a browser load: a bounded, allocation-light
//! scan of HTML tags and CSS `url()`/`@import`, not a full HTML parser.
//!
//! HTML: `<link rel=stylesheet|icon|manifest|preload|modulepreload>`,
//! `<script src>`, `<img src>` (else the first `srcset` candidate),
//! `<video poster>`, `<iframe src>`, `<base href>`, `<link rel=preconnect>`
//! and `dns-prefetch` (an origin, never a request), and `url()` in `<style>`
//! elements and `style` attributes. Comments and the contents of `<script>`,
//! `<template>`, `<noscript>`, `<textarea>` and `<title>` are skipped.
//! Media sources (`<video>`, `<audio>`, `<source>`), `prefetch` and links
//! are not resources of the page and are left out.
//!
//! CSS: every `url()` and `@import`, except that a `src` declaration (an
//! `@font-face` source list) counts only its first `url()`, the one a
//! browser that supports the first format downloads.

/// What a reference loads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Kind {
    Document,
    Stylesheet,
    Script,
    Font,
    Image,
    Manifest,
    Other,
    /// `preconnect` or `dns-prefetch`: an origin, never fetched.
    Connection,
}

impl Kind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::Stylesheet => "stylesheet",
            Self::Script => "script",
            Self::Font => "font",
            Self::Image => "image",
            Self::Manifest => "manifest",
            Self::Other => "other",
            Self::Connection => "connection",
        }
    }
}

/// One URL as written in the page, not yet resolved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Reference {
    pub url: String,
    pub kind: Kind,
}

impl Reference {
    fn new(url: &str, kind: Kind) -> Option<Self> {
        let url = url.trim();
        (!url.is_empty()).then(|| Self {
            url: url.to_string(),
            kind,
        })
    }
}

/// The references of an HTML document and its `<base href>`, if any.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct HtmlScan {
    pub base: Option<String>,
    pub references: Vec<Reference>,
}

/// Elements whose contents are not markup a browser loads resources from.
const RAW_TEXT: &[&str] = &[
    "script", "style", "template", "noscript", "textarea", "title",
];

struct Tag<'a> {
    name: &'a str,
    attributes: Vec<(&'a str, String)>,
    /// The byte after the tag's `>`.
    end: usize,
}

impl Tag<'_> {
    fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

fn find(bytes: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    bytes
        .get(from..)?
        .windows(needle.len())
        .position(|window| window.eq_ignore_ascii_case(needle))
        .map(|offset| from + offset)
}

/// Whether `needle` (ignoring ASCII case) starts at `index`.
fn starts_at(bytes: &[u8], index: usize, needle: &[u8]) -> bool {
    bytes
        .get(index..index + needle.len())
        .is_some_and(|window| window.eq_ignore_ascii_case(needle))
}

fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | b'\x0c')
}

/// Replaces the character references URLs commonly carry.
fn decode(value: &str) -> String {
    if !value.contains('&') {
        return value.to_string();
    }
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        let Some(end) = rest.find(';').filter(|end| *end <= 10) else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let entity = &rest[1..end];
        let decoded = match entity {
            "amp" => Some('&'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "lt" => Some('<'),
            "gt" => Some('>'),
            _ => entity
                .strip_prefix("#x")
                .or_else(|| entity.strip_prefix("#X"))
                .map(|hex| u32::from_str_radix(hex, 16))
                .or_else(|| entity.strip_prefix('#').map(str::parse::<u32>))
                .and_then(Result::ok)
                .and_then(char::from_u32),
        };
        match decoded {
            Some(character) => {
                out.push(character);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// The tag starting at the `<` at `start`, or `None` when no tag name
/// follows (text, an end tag, a comment or a declaration).
fn tag(html: &str, start: usize) -> Option<Tag<'_>> {
    let bytes = html.as_bytes();
    let mut index = start + 1;
    if !bytes.get(index)?.is_ascii_alphabetic() {
        return None;
    }
    while bytes
        .get(index)
        .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'-')
    {
        index += 1;
    }
    let name = &html[start + 1..index];
    let mut attributes = Vec::new();
    loop {
        while bytes
            .get(index)
            .is_some_and(|byte| is_space(*byte) || *byte == b'/')
        {
            index += 1;
        }
        match bytes.get(index) {
            None => {
                return Some(Tag {
                    name,
                    attributes,
                    end: bytes.len(),
                });
            }
            Some(b'>') => {
                return Some(Tag {
                    name,
                    attributes,
                    end: index + 1,
                });
            }
            Some(_) => {}
        }
        let key_start = index;
        while bytes
            .get(index)
            .is_some_and(|byte| !is_space(*byte) && !matches!(byte, b'=' | b'>' | b'/'))
        {
            index += 1;
        }
        let key = &html[key_start..index];
        while bytes.get(index).is_some_and(|byte| is_space(*byte)) {
            index += 1;
        }
        if bytes.get(index) != Some(&b'=') {
            attributes.push((key, String::new()));
            continue;
        }
        index += 1;
        while bytes.get(index).is_some_and(|byte| is_space(*byte)) {
            index += 1;
        }
        let value = match bytes.get(index) {
            Some(quote @ (b'"' | b'\'')) => {
                let value_start = index + 1;
                let value_end = bytes
                    .get(value_start..)
                    .and_then(|rest| rest.iter().position(|byte| byte == quote))
                    .map_or(bytes.len(), |offset| value_start + offset);
                index = (value_end + 1).min(bytes.len());
                &html[value_start..value_end]
            }
            _ => {
                let value_start = index;
                while bytes
                    .get(index)
                    .is_some_and(|byte| !is_space(*byte) && *byte != b'>')
                {
                    index += 1;
                }
                &html[value_start..index]
            }
        };
        attributes.push((key, decode(value)));
    }
}

/// The first URL of a `srcset` list.
fn first_candidate(srcset: &str) -> Option<&str> {
    srcset.split(',').next()?.split_whitespace().next()
}

fn link(tag: &Tag<'_>, references: &mut Vec<Reference>) {
    let rel = tag
        .attribute("rel")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let rels: Vec<&str> = rel.split_ascii_whitespace().collect();
    let has = |name: &str| rels.contains(&name);
    let kind = if has("stylesheet") && !has("alternate") {
        Some(Kind::Stylesheet)
    } else if has("modulepreload") {
        Some(Kind::Script)
    } else if has("preload") {
        Some(
            match tag
                .attribute("as")
                .unwrap_or_default()
                .to_ascii_lowercase()
                .as_str()
            {
                "style" => Kind::Stylesheet,
                "script" => Kind::Script,
                "font" => Kind::Font,
                "image" => Kind::Image,
                _ => Kind::Other,
            },
        )
    } else if has("icon") {
        Some(Kind::Image)
    } else if has("manifest") {
        Some(Kind::Manifest)
    } else if has("preconnect") || has("dns-prefetch") {
        Some(Kind::Connection)
    } else {
        None
    };
    if let Some(kind) = kind {
        references.extend(Reference::new(
            tag.attribute("href").unwrap_or_default(),
            kind,
        ));
    }
}

/// The references of `html`, in document order.
pub(crate) fn html(html: &str) -> HtmlScan {
    let bytes = html.as_bytes();
    let mut scan = HtmlScan::default();
    let mut index = 0;
    while let Some(start) = bytes
        .get(index..)
        .and_then(|rest| rest.iter().position(|byte| *byte == b'<'))
        .map(|offset| index + offset)
    {
        if starts_at(bytes, start, b"<!--") {
            index = find(bytes, start + 4, b"-->").map_or(bytes.len(), |end| end + 3);
            continue;
        }
        let Some(tag) = tag(html, start) else {
            index = start + 1;
            continue;
        };
        index = tag.end;
        let references = &mut scan.references;
        let attribute = |name: &str| tag.attribute(name).unwrap_or_default();
        match tag.name.to_ascii_lowercase().as_str() {
            "link" => link(&tag, references),
            "script" => references.extend(Reference::new(attribute("src"), Kind::Script)),
            "img" => {
                let source = tag
                    .attribute("src")
                    .filter(|source| !source.trim().is_empty())
                    .or_else(|| tag.attribute("srcset").and_then(first_candidate));
                references.extend(source.and_then(|source| Reference::new(source, Kind::Image)));
            }
            "video" => references.extend(Reference::new(attribute("poster"), Kind::Image)),
            "iframe" => references.extend(Reference::new(attribute("src"), Kind::Document)),
            "base" if scan.base.is_none() => {
                scan.base =
                    Some(attribute("href").trim().to_string()).filter(|base| !base.is_empty());
            }
            _ => {}
        }
        if let Some(style) = tag.attribute("style") {
            scan.references.extend(css(style));
        }
        let name = tag.name;
        if RAW_TEXT.iter().any(|raw| raw.eq_ignore_ascii_case(name)) {
            let closing = format!("</{name}");
            let end = find(bytes, index, closing.as_bytes()).unwrap_or(bytes.len());
            if name.eq_ignore_ascii_case("style") {
                scan.references.extend(css(&html[index..end]));
            }
            index = end;
        }
    }
    scan
}

/// The kind of a URL a stylesheet references, from its extension.
fn css_kind(url: &str) -> Kind {
    let path = url.split(['?', '#']).next().unwrap_or_default();
    let extension = path
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .unwrap_or_default();
    match extension.as_str() {
        "woff" | "woff2" | "ttf" | "otf" | "eot" => Kind::Font,
        "css" => Kind::Stylesheet,
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "avif" | "svg" | "ico" | "bmp" => Kind::Image,
        _ => Kind::Other,
    }
}

/// The argument of the `url(` whose argument starts at `start`, and the
/// byte after its `)`.
fn url_argument(css: &str, start: usize) -> (String, usize) {
    let bytes = css.as_bytes();
    let mut index = start;
    while bytes.get(index).is_some_and(|byte| is_space(*byte)) {
        index += 1;
    }
    if let Some(quote @ (b'"' | b'\'')) = bytes.get(index) {
        let value_start = index + 1;
        let value_end = bytes
            .get(value_start..)
            .and_then(|rest| rest.iter().position(|byte| byte == quote))
            .map_or(bytes.len(), |offset| value_start + offset);
        let close = bytes
            .get(value_end..)
            .and_then(|rest| rest.iter().position(|byte| *byte == b')'))
            .map_or(bytes.len(), |offset| value_end + offset + 1);
        return (css[value_start..value_end].to_string(), close);
    }
    let close = bytes
        .get(index..)
        .and_then(|rest| rest.iter().position(|byte| *byte == b')'))
        .map_or(bytes.len(), |offset| index + offset);
    (
        css[index..close].trim().to_string(),
        (close + 1).min(bytes.len()),
    )
}

/// The string after `@import` at `start`: `"x.css"` or `url(x.css)`.
fn import_argument(css: &str, start: usize) -> Option<(String, usize)> {
    let bytes = css.as_bytes();
    let mut index = start;
    while bytes.get(index).is_some_and(|byte| is_space(*byte)) {
        index += 1;
    }
    match bytes.get(index)? {
        quote @ (b'"' | b'\'') => {
            let value_start = index + 1;
            let value_end = bytes
                .get(value_start..)?
                .iter()
                .position(|byte| byte == quote)
                .map(|offset| value_start + offset)?;
            Some((css[value_start..value_end].to_string(), value_end + 1))
        }
        _ if starts_at(bytes, index, b"url(") => Some(url_argument(css, index + 4)),
        _ => None,
    }
}

/// The references of a stylesheet (or an inline `style`), in order.
pub(crate) fn css(css: &str) -> Vec<Reference> {
    let bytes = css.as_bytes();
    let mut references = Vec::new();
    let mut index = 0;
    // Where the current declaration started, and whether it already gave
    // its one counted `src` URL.
    let mut declaration = 0;
    let mut source_taken = false;
    while let Some(&byte) = bytes.get(index) {
        match byte {
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                index = find(bytes, index + 2, b"*/").map_or(bytes.len(), |end| end + 2);
            }
            b'"' | b'\'' => {
                index = bytes
                    .get(index + 1..)
                    .and_then(|rest| rest.iter().position(|next| *next == byte))
                    .map_or(bytes.len(), |offset| index + offset + 2);
            }
            b'{' | b'}' | b';' => {
                index += 1;
                declaration = index;
                source_taken = false;
            }
            b'@' if starts_at(bytes, index, b"@import") => match import_argument(css, index + 7) {
                Some((url, end)) => {
                    references.extend(Reference::new(&url, Kind::Stylesheet));
                    index = end;
                }
                None => index += 7,
            },
            b'u' | b'U' if starts_at(bytes, index, b"url(") => {
                let (url, end) = url_argument(css, index + 4);
                let property = css[declaration..index]
                    .split(':')
                    .next()
                    .unwrap_or_default()
                    .trim();
                let source = property.eq_ignore_ascii_case("src");
                if !(source && source_taken) && !url.starts_with('#') {
                    references.extend(Reference::new(&url, css_kind(&url)));
                }
                source_taken |= source;
                index = end;
            }
            _ => index += 1,
        }
    }
    references
}

#[cfg(test)]
#[path = "scan_tests.rs"]
mod tests;
