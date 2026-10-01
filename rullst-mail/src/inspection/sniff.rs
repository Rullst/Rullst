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
/// host on open, including the executable and script types of Outlook's
/// Level 1 blocked list. Rejected under every policy, like executable magic.
const EXECUTABLE_EXTENSIONS: &[&str] = &[
    "exe",
    "com",
    "scr",
    "pif",
    "cpl",
    "msi",
    "msp",
    "mst",
    "msu",
    "bat",
    "cmd",
    "ps1",
    "psm1",
    "psd1",
    "ps1xml",
    "ps2",
    "ps2xml",
    "psc1",
    "psc2",
    "psdm1",
    "msh",
    "msh1",
    "msh2",
    "mshxml",
    "msh1xml",
    "msh2xml",
    "vb",
    "vbs",
    "vbe",
    "vbp",
    "js",
    "jse",
    "ws",
    "wsf",
    "wsh",
    "wsc",
    "sct",
    "hta",
    "lnk",
    "reg",
    "inf",
    "ins",
    "isp",
    "jar",
    "jnlp",
    "chm",
    "dll",
    "scf",
    "url",
    "website",
    "msc",
    "appref-ms",
    "application",
    "xbap",
    "gadget",
    "settingcontent-ms",
    "xll",
    "ade",
    "adp",
    "mde",
    "bas",
    "diagcab",
    "vhd",
    "vhdx",
    "wsb",
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

/// Classifies leading markup and script URIs, as rullst-core uploads do.
pub(super) fn markup(content: &[u8]) -> Option<Markup> {
    let body = content.strip_prefix(b"\xef\xbb\xbf").unwrap_or(content);
    let head = body.trim_ascii_start();
    let starts = |prefix: &[u8]| {
        head.get(..prefix.len())
            .is_some_and(|start| start.eq_ignore_ascii_case(prefix))
    };
    // An XML declaration or leading comment can precede the real root element.
    let prologue = starts(b"<?xml") || starts(b"<!--");
    if starts(b"<svg")
        || ((prologue || starts(b"<!doctype svg"))
            && contains_ascii_case_insensitive(body, b"<svg"))
    {
        return Some(Markup::Svg);
    }
    if ACTIVE_PREFIXES.iter().any(|prefix| starts(prefix))
        || (prologue
            && [b"<html".as_slice(), b"<script", b"/1999/xhtml"]
                .iter()
                .any(|needle| contains_ascii_case_insensitive(body, needle)))
        || contains_ascii_case_insensitive(body, b"javascript:")
        || contains_ascii_case_insensitive(body, b"vbscript:")
    {
        return Some(Markup::Active);
    }
    None
}

pub(super) fn contains_ascii_case_insensitive(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|window| {
        window
            .iter()
            .zip(needle)
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
    })
}
