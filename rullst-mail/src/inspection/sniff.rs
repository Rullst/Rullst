//! Filename and content sniffing, so inspection never relies on the declared
//! MIME type alone. Mail clients pick a handler from the extension and some
//! readers accept a format by its signature, whatever the label says.

/// Content families the local inspector knows how to check.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Kind {
    /// Any UTF-8 text format inspected by the text heuristics.
    Text,
    Pdf,
    Png,
    Jpeg,
    Gif,
    Svg,
    Zip,
}

impl Kind {
    /// Maps a declared MIME type, compared case-insensitively.
    pub(super) fn from_mime(mime: &str) -> Option<Self> {
        match mime.to_ascii_lowercase().as_str() {
            "text/plain" | "text/csv" | "application/json" | "application/xml" => Some(Self::Text),
            "application/pdf" => Some(Self::Pdf),
            "image/png" => Some(Self::Png),
            "image/jpeg" => Some(Self::Jpeg),
            "image/gif" => Some(Self::Gif),
            "image/svg+xml" => Some(Self::Svg),
            "application/zip" => Some(Self::Zip),
            _ => None,
        }
    }

    /// Maps a lowercase filename extension.
    pub(super) fn from_extension(extension: &str) -> Option<Self> {
        match extension {
            "txt" | "text" | "log" | "md" | "csv" | "json" | "xml" => Some(Self::Text),
            "pdf" => Some(Self::Pdf),
            "png" => Some(Self::Png),
            "jpg" | "jpeg" => Some(Self::Jpeg),
            "gif" => Some(Self::Gif),
            "svg" | "svgz" => Some(Self::Svg),
            "zip" => Some(Self::Zip),
            _ => None,
        }
    }
}

/// Extensions that Windows or common mail clients execute or hand to a script
/// host on open. Rejected under every policy, like executable magic.
const EXECUTABLE_EXTENSIONS: &[&str] = &[
    "exe", "com", "scr", "pif", "cpl", "msi", "msp", "bat", "cmd", "ps1", "psm1", "vbs", "vbe",
    "js", "jse", "wsf", "wsh", "hta", "lnk", "reg", "jar", "chm", "dll", "scf", "url",
];

/// Extensions opened by a browser engine. Rejected by the strict policy.
const HTML_EXTENSIONS: &[&str] = &["html", "htm", "xhtml", "xht", "shtml", "mht", "mhtml"];

/// Leading markup that makes a text payload an active document.
const ACTIVE_PREFIXES: &[&[u8]] = &[
    b"<html",
    b"<!doctype html",
    b"<script",
    b"<hta:",
    b"<head",
    b"<body",
    b"<iframe",
];

/// Returns the lowercase final extension. Trailing dots and spaces are
/// ignored because Windows strips them when the file is saved.
pub(super) fn extension(filename: &str) -> Option<String> {
    let (_, extension) = filename.trim_end_matches(['.', ' ']).rsplit_once('.')?;
    (!extension.is_empty()).then(|| extension.to_ascii_lowercase())
}

pub(super) fn is_executable_extension(extension: &str) -> bool {
    EXECUTABLE_EXTENSIONS.contains(&extension)
}

pub(super) fn is_html_extension(extension: &str) -> bool {
    HTML_EXTENSIONS.contains(&extension)
}

/// PDF readers accept a `%PDF-` header anywhere in the first KiB.
pub(super) fn looks_like_pdf(content: &[u8]) -> bool {
    content
        .get(..1024)
        .unwrap_or(content)
        .windows(5)
        .any(|window| window == b"%PDF-")
}

/// Markup detected from content, independent of the declared type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Markup {
    /// An SVG document, which can carry script and event handlers.
    Svg,
    /// HTML, HTA or script markup, or an embedded script URI.
    Active,
}

/// Browsers build SVG elements, which run script, from this namespace
/// whatever prefix the document binds it to.
const SVG_NAMESPACE: &[u8] = b"http://www.w3.org/2000/svg";
/// The distinctive path of the XHTML namespace, `http://www.w3.org/1999/xhtml`.
const XHTML_NAMESPACE: &[u8] = b"/1999/xhtml";

/// Root element names, after any `prefix:`, of an active document.
const ACTIVE_ROOTS: &[&[u8]] = &[b"html", b"script", b"head", b"body", b"iframe"];

/// Classifies leading markup and script URIs, as rullst-core uploads do.
///
/// A document that opens with markup is classified the way a browser parses
/// it: by the root element after any XML declaration, processing
/// instruction, comment or DOCTYPE, matched after an optional `prefix:`, and
/// by the SVG and XHTML namespace URIs anywhere in it, also when they are
/// spelled with character references. A document declaring DTD entities,
/// which can assemble a namespace URI from pieces, is treated as SVG unless
/// its root is `html`. A UTF-16 document is read by its byte-order mark.
pub(super) fn markup(content: &[u8]) -> Option<Markup> {
    match utf16_text(content) {
        Some(text) => utf8_markup(text.as_bytes()),
        None => utf8_markup(content),
    }
}

fn utf8_markup(content: &[u8]) -> Option<Markup> {
    let body = content.strip_prefix(b"\xef\xbb\xbf").unwrap_or(content);
    let head = body.trim_ascii_start();
    let starts = |prefix: &[u8]| {
        head.get(..prefix.len())
            .is_some_and(|start| start.eq_ignore_ascii_case(prefix))
    };
    // An XML declaration or leading comment can precede the real root element.
    let prologue = starts(b"<?xml") || starts(b"<!--");
    let document = head.first() == Some(&b'<');
    let decoded = if document {
        crate::entities::decode(&String::from_utf8_lossy(body)).into_owned()
    } else {
        String::new()
    };
    let declares = |needle: &[u8]| {
        document
            && (contains_ascii_case_insensitive(body, needle)
                || contains_ascii_case_insensitive(decoded.as_bytes(), needle))
    };
    let (root_prefix, root) = root_element(head).map_or((None, None), |name| {
        match name.iter().rposition(|byte| *byte == b':') {
            Some(colon) => (Some(&name[..colon]), Some(&name[colon + 1..])),
            None => (None, Some(name)),
        }
    });
    let root_is = |name: &[u8]| root.is_some_and(|root| root.eq_ignore_ascii_case(name));
    let entities = document && contains_ascii_case_insensitive(body, b"<!entity");
    if starts(b"<svg")
        || ((prologue || starts(b"<!doctype svg"))
            && contains_ascii_case_insensitive(body, b"<svg"))
        || root_is(b"svg")
        || (!root_is(b"html") && (declares(SVG_NAMESPACE) || entities))
    {
        return Some(Markup::Svg);
    }
    if ACTIVE_PREFIXES.iter().any(|prefix| starts(prefix))
        || (prologue
            && [b"<html".as_slice(), b"<script"]
                .iter()
                .any(|needle| contains_ascii_case_insensitive(body, needle)))
        || ACTIVE_ROOTS.iter().any(|name| root_is(name))
        || root_prefix.is_some_and(|prefix| prefix.eq_ignore_ascii_case(b"hta"))
        || declares(XHTML_NAMESPACE)
        || contains_ascii_case_insensitive(body, b"javascript:")
        || contains_ascii_case_insensitive(body, b"vbscript:")
    {
        return Some(Markup::Active);
    }
    None
}

/// Decodes content that starts with a UTF-16 byte-order mark, as browsers do.
fn utf16_text(content: &[u8]) -> Option<String> {
    let (rest, little_endian) = if let Some(rest) = content.strip_prefix(b"\xff\xfe") {
        (rest, true)
    } else {
        (content.strip_prefix(b"\xfe\xff")?, false)
    };
    let units = rest.as_chunks::<2>().0.iter().map(|&pair| {
        if little_endian {
            u16::from_le_bytes(pair)
        } else {
            u16::from_be_bytes(pair)
        }
    });
    Some(
        char::decode_utf16(units)
            .map(|unit| unit.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect(),
    )
}

/// The qualified name of the first element after any XML declaration,
/// processing instructions, comments and DOCTYPE (with its internal subset).
fn root_element(mut rest: &[u8]) -> Option<&[u8]> {
    let after = |haystack: &[u8], needle: &[u8]| {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
            .map(|index| index + needle.len())
    };
    loop {
        rest = rest.trim_ascii_start();
        let skip = if rest.starts_with(b"<?") {
            after(rest, b"?>")?
        } else if rest.starts_with(b"<!--") {
            4 + after(rest.get(4..)?, b"-->")?
        } else if rest.starts_with(b"<!") {
            // A DOCTYPE internal subset may hold `>` before the closing one.
            let close = after(rest, b">")?;
            match after(rest, b"[") {
                Some(open) if open < close => {
                    let subset_end = open + after(rest.get(open..)?, b"]")?;
                    subset_end + after(rest.get(subset_end..)?, b">")?
                }
                _ => close,
            }
        } else {
            let name = rest.strip_prefix(b"<")?;
            let length = name
                .iter()
                .position(|byte| byte.is_ascii_whitespace() || matches!(byte, b'>' | b'/'))
                .unwrap_or(name.len());
            return Some(&name[..length]);
        };
        rest = rest.get(skip..)?;
    }
}

pub(super) fn contains_ascii_case_insensitive(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|window| {
        window
            .iter()
            .zip(needle)
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
    })
}
