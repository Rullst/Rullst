//! Bounded reader for `multipart/byteranges` response bodies (RFC 9110
//! section 14.6), so the DLP layer can inspect every range of a multi-range
//! `206` by the media type of its own part.
//!
//! The reader never rewrites a body; it only splits it. Anything it cannot
//! split exactly is [`Malformed`], and the caller withholds the response: a
//! missing or invalid boundary, a line that starts with the dash boundary but
//! is not a delimiter, a part header block over [`MAX_PART_HEADER_BYTES`], an
//! invalid header field, an empty or repeated `Content-Type`, more than
//! [`MAX_PARTS`] parts or a body without its close delimiter. Line breaks may
//! be CRLF or a bare LF. The body is scanned once, so the cost is linear in
//! its length.

use crate::media_type::essence;

/// Most parts read from one response.
const MAX_PARTS: usize = 256;
/// Longest header block of one part, including its line breaks.
const MAX_PART_HEADER_BYTES: usize = 8 * 1024;
/// Media type of a part without `Content-Type` (RFC 2046 section 5.1).
const DEFAULT_PART_MEDIA_TYPE: &str = "text/plain";

/// Whether a `Content-Type` media type is `multipart/byteranges`.
pub(super) fn is_byteranges(media_type: &str) -> bool {
    media_type.eq_ignore_ascii_case("multipart/byteranges")
}

/// The body could not be split exactly into its parts.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Malformed;

/// Whether masking would change any range of a `multipart/byteranges` body.
///
/// Each identity-encoded part whose media type is textual is checked; other
/// parts pass, as such responses do. The preamble and epilogue are checked as
/// text.
pub(super) fn needs_masking(content_type: &str, body: &[u8]) -> Result<bool, Malformed> {
    let split = split(content_type, body)?;
    let outside = split.outside.iter().any(|bytes| would_mask(bytes));
    Ok(outside
        || split.parts.iter().any(|part| {
            let media_type = part.content_type.map_or(DEFAULT_PART_MEDIA_TYPE, essence);
            part.identity_encoded
                && super::is_textual_media_type(media_type)
                && would_mask(part.body)
        }))
}

fn would_mask(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes).is_ok_and(|text| super::mask_text(text).is_some())
}

/// Returns the boundary of a `multipart/byteranges` content type.
///
/// The boundary must satisfy RFC 2046: 1-70 characters from its `bchars`
/// set, not ending in a space. A malformed or repeated parameter fails.
fn boundary(content_type: &str) -> Option<&str> {
    let mut parameters = content_type.split(';');
    if !is_byteranges(parameters.next()?.trim()) {
        return None;
    }
    let mut boundary = None;
    for parameter in parameters.filter(|parameter| !parameter.trim().is_empty()) {
        let (name, value) = parameter.split_once('=')?;
        if !name.trim().eq_ignore_ascii_case("boundary") {
            continue;
        }
        if boundary.is_some() {
            return None;
        }
        let value = value.trim();
        boundary = Some(
            value
                .strip_prefix('"')
                .and_then(|quoted| quoted.strip_suffix('"'))
                .unwrap_or(value),
        );
    }
    let boundary = boundary?;
    let valid = (1..=70).contains(&boundary.len())
        && !boundary.ends_with(' ')
        && boundary
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"'()+_,-./:=? ".contains(&byte));
    valid.then_some(boundary)
}

/// One body part.
struct Part<'a> {
    content_type: Option<&'a str>,
    /// The part has no `Content-Encoding` other than `identity`.
    identity_encoded: bool,
    body: &'a [u8],
}

/// A body split into its parts and the bytes outside them.
struct Byteranges<'a> {
    parts: Vec<Part<'a>>,
    /// The preamble before the first delimiter and the epilogue after the
    /// close delimiter.
    outside: [&'a [u8]; 2],
}

/// Splits a `multipart/byteranges` body whose `Content-Type` value is
/// `content_type`.
fn split<'a>(content_type: &str, body: &'a [u8]) -> Result<Byteranges<'a>, Malformed> {
    let boundary = boundary(content_type).ok_or(Malformed)?.as_bytes();
    let mut lines = Lines { body, position: 0 };
    let preamble_end = loop {
        let line = lines.next().ok_or(Malformed)?;
        match delimiter(line.text(body), boundary)? {
            Some(Delimiter::Part) => break line.start,
            Some(Delimiter::Close) => return Err(Malformed),
            None => {}
        }
    };

    let mut parts = Vec::new();
    loop {
        if parts.len() == MAX_PARTS {
            return Err(Malformed);
        }
        let (content_type, identity_encoded) = read_part_headers(&mut lines)?;
        let content_start = lines.position;
        let (close, next) = loop {
            let line = lines.next().ok_or(Malformed)?;
            if let Some(kind) = delimiter(line.text(body), boundary)? {
                let end = content_end(body, content_start, line.start);
                parts.push(Part {
                    content_type,
                    identity_encoded,
                    body: body.get(content_start..end).ok_or(Malformed)?,
                });
                break (kind == Delimiter::Close, line.next);
            }
        };
        if close {
            let preamble = body.get(..preamble_end).ok_or(Malformed)?;
            let epilogue = body.get(next..).ok_or(Malformed)?;
            return Ok(Byteranges {
                parts,
                outside: [preamble, epilogue],
            });
        }
    }
}

/// Reads one part's header block, up to and including its empty line, and
/// returns its `Content-Type` and whether it is identity-encoded.
fn read_part_headers<'a>(lines: &mut Lines<'a>) -> Result<(Option<&'a str>, bool), Malformed> {
    let body = lines.body;
    let start = lines.position;
    let mut content_type = None;
    let mut identity_encoded = true;
    loop {
        let line = lines.next().ok_or(Malformed)?;
        if line.next - start > MAX_PART_HEADER_BYTES {
            return Err(Malformed);
        }
        let field = line.text(body);
        if field.is_empty() {
            return Ok((content_type, identity_encoded));
        }
        let field = std::str::from_utf8(field).map_err(|_| Malformed)?;
        let (name, value) = field.split_once(':').ok_or(Malformed)?;
        // A field name is a token: no whitespace and no obsolete line folding.
        if name.is_empty() || name.bytes().any(|byte| byte.is_ascii_whitespace()) {
            return Err(Malformed);
        }
        let value = value.trim();
        if name.eq_ignore_ascii_case("content-type") {
            if value.is_empty() || content_type.replace(value).is_some() {
                return Err(Malformed);
            }
        } else if name.eq_ignore_ascii_case("content-encoding") {
            if value.is_empty() {
                return Err(Malformed);
            }
            identity_encoded &= value.eq_ignore_ascii_case("identity");
        }
    }
}

/// End of a part's content: the line break before a delimiter belongs to the
/// delimiter (RFC 2046 section 5.1.1).
fn content_end(body: &[u8], content_start: usize, delimiter_start: usize) -> usize {
    let content = body.get(content_start..delimiter_start).unwrap_or_default();
    let content = content
        .strip_suffix(b"\n")
        .map_or(content, |rest| rest.strip_suffix(b"\r").unwrap_or(rest));
    content_start + content.len()
}

#[derive(Debug, PartialEq, Eq)]
enum Delimiter {
    /// `--boundary`: another part follows.
    Part,
    /// `--boundary--`: the last part has ended.
    Close,
}

/// Classifies a line. Transport padding (spaces and tabs) may follow a
/// delimiter; any other line that starts with the dash boundary is ambiguous.
fn delimiter(line: &[u8], boundary: &[u8]) -> Result<Option<Delimiter>, Malformed> {
    let Some(rest) = line
        .strip_prefix(b"--")
        .and_then(|rest| rest.strip_prefix(boundary))
    else {
        return Ok(None);
    };
    let (kind, padding) = match rest.strip_prefix(b"--") {
        Some(padding) => (Delimiter::Close, padding),
        None => (Delimiter::Part, rest),
    };
    if padding.iter().all(|byte| matches!(byte, b' ' | b'\t')) {
        Ok(Some(kind))
    } else {
        Err(Malformed)
    }
}

/// One line: `start..end` excludes its line break, `next` starts the next line.
struct Line {
    start: usize,
    end: usize,
    next: usize,
}

impl Line {
    fn text<'a>(&self, body: &'a [u8]) -> &'a [u8] {
        body.get(self.start..self.end).unwrap_or_default()
    }
}

struct Lines<'a> {
    body: &'a [u8],
    position: usize,
}

impl Iterator for Lines<'_> {
    type Item = Line;

    fn next(&mut self) -> Option<Line> {
        let start = self.position;
        let rest = self.body.get(start..).filter(|rest| !rest.is_empty())?;
        let (mut end, next) = match rest.iter().position(|byte| *byte == b'\n') {
            Some(offset) => (start + offset, start + offset + 1),
            None => (self.body.len(), self.body.len()),
        };
        if end > start && self.body.get(end - 1) == Some(&b'\r') {
            end -= 1;
        }
        self.position = next;
        Some(Line { start, end, next })
    }
}

#[cfg(test)]
#[path = "byteranges_tests.rs"]
mod tests;
