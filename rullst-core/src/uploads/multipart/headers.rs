//! Part headers of a `multipart/form-data` body (RFC 7578).

use super::MultipartError;
use crate::uploads::file_names::is_hidden_format;

/// Largest header block of one part.
pub(super) const MAX_PART_HEADER_BYTES: usize = 8 * 1024;

/// Longest display name kept from a client file name.
const MAX_DISPLAY_NAME_BYTES: usize = 255;

/// The headers of one part.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct PartHeaders {
    /// The `name` parameter of `Content-Disposition`.
    pub(super) name: String,
    /// Present for a file part (one with `filename` or `filename*`).
    pub(super) file: Option<FileMeta>,
}

/// What a file part says about itself; none of it is trusted.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct FileMeta {
    /// The sanitized client file name.
    pub(super) display_name: Option<String>,
    /// The declared `Content-Type`, when printable.
    pub(super) content_type: Option<String>,
    /// `filename=""`: a browser's empty file input.
    pub(super) empty_file_name: bool,
}

fn malformed(reason: &'static str) -> MultipartError {
    MultipartError::Malformed(reason)
}

/// Parses the header block of one part (without its final empty line).
pub(super) fn parse(raw: &[u8], max_name_bytes: usize) -> Result<PartHeaders, MultipartError> {
    let text = std::str::from_utf8(raw).map_err(|_| malformed("part headers are not UTF-8"))?;
    let mut disposition = None;
    let mut content_type = None;
    for line in text.split("\r\n").filter(|_| !text.is_empty()) {
        let (name, value) = line
            .split_once(':')
            .ok_or(malformed("a part header has no colon"))?;
        if name.is_empty() || !name.bytes().all(is_token_byte) {
            return Err(malformed("a part header name is not a token"));
        }
        let slot = if name.eq_ignore_ascii_case("content-disposition") {
            &mut disposition
        } else if name.eq_ignore_ascii_case("content-type") {
            &mut content_type
        } else {
            continue;
        };
        if slot.replace(value.trim()).is_some() {
            return Err(malformed("a part repeats a header"));
        }
    }
    let disposition =
        parse_disposition(disposition.ok_or(malformed("a part has no Content-Disposition"))?)?;
    if content_type.is_some_and(|value| {
        value
            .get(..10)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("multipart/"))
    }) {
        return Err(malformed("nested multipart parts are not supported"));
    }
    let name = disposition
        .name
        .filter(|name| !name.is_empty())
        .ok_or(malformed("a part has no field name"))?;
    if name.len() > max_name_bytes {
        return Err(MultipartError::FieldNameTooLong);
    }
    let file =
        (disposition.file_name.is_some() || disposition.extended_file_name).then(|| FileMeta {
            empty_file_name: !disposition.extended_file_name
                && disposition.file_name.as_deref() == Some(""),
            display_name: disposition.file_name.as_deref().and_then(display_name),
            content_type: content_type.and_then(display_content_type),
        });
    Ok(PartHeaders { name, file })
}

/// RFC 9110 `tchar`.
fn is_token_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte)
}

/// The parameters of a `form-data` disposition that a part may give once.
#[derive(Debug, Default, PartialEq, Eq)]
struct Disposition {
    name: Option<String>,
    file_name: Option<String>,
    extended_file_name: bool,
}

fn parse_disposition(value: &str) -> Result<Disposition, MultipartError> {
    let (kind, mut rest) = value.split_at(value.find(';').unwrap_or(value.len()));
    if !kind.trim().eq_ignore_ascii_case("form-data") {
        return Err(malformed("a part is not form-data"));
    }
    let mut disposition = Disposition::default();
    loop {
        rest = rest.trim_start();
        let Some(after) = rest.strip_prefix(';') else {
            if rest.is_empty() {
                return Ok(disposition);
            }
            return Err(malformed("malformed Content-Disposition parameters"));
        };
        rest = after.trim_start();
        if rest.is_empty() {
            return Ok(disposition);
        }
        let (name, after) = rest
            .split_once('=')
            .ok_or(malformed("malformed Content-Disposition parameters"))?;
        let name = name.trim_end();
        if name.is_empty() || !name.bytes().all(is_token_byte) {
            return Err(malformed("malformed Content-Disposition parameters"));
        }
        let after = after.trim_start();
        let value = if let Some(quoted) = after.strip_prefix('"') {
            let (value, remaining) = parse_quoted(quoted)?;
            rest = remaining;
            value
        } else {
            let end = after.find(';').unwrap_or(after.len());
            rest = &after[end..];
            after[..end].trim_end().to_owned()
        };
        let repeated = if name.eq_ignore_ascii_case("name") {
            disposition.name.replace(value).is_some()
        } else if name.eq_ignore_ascii_case("filename") {
            disposition.file_name.replace(value).is_some()
        } else if name.eq_ignore_ascii_case("filename*") {
            std::mem::replace(&mut disposition.extended_file_name, true)
        } else {
            false
        };
        if repeated {
            return Err(malformed("a Content-Disposition parameter is repeated"));
        }
    }
}

/// Reads a quoted string after its opening quote; returns the value and the
/// text after the closing quote. `\"` and `\\` are escapes (curl sends
/// them); any other backslash is kept, as in an old browser's Windows path.
fn parse_quoted(input: &str) -> Result<(String, &str), MultipartError> {
    let mut value = String::new();
    let mut characters = input.char_indices();
    while let Some((index, character)) = characters.next() {
        match character {
            '"' => return Ok((value, &input[index + 1..])),
            '\\' => match characters.clone().next() {
                Some((_, escaped @ ('"' | '\\'))) => {
                    value.push(escaped);
                    characters.next();
                }
                _ => value.push('\\'),
            },
            _ => value.push(character),
        }
    }
    Err(malformed("a Content-Disposition value is not closed"))
}

/// The display-only form of a client file name: its last `/` or `\`
/// segment without control or invisible format characters, trimmed and cut
/// to 255 bytes; `None` when nothing (or only `.` or `..`) is left.
pub(super) fn display_name(raw: &str) -> Option<String> {
    let last = raw.rsplit(['/', '\\']).next().unwrap_or(raw);
    let visible = last
        .chars()
        .filter(|&character| !character.is_control() && !is_hidden_format(character))
        .collect::<String>();
    let trimmed = visible.trim();
    if trimmed.is_empty() || trimmed == "." || trimmed == ".." {
        return None;
    }
    let mut end = trimmed.len().min(MAX_DISPLAY_NAME_BYTES);
    while !trimmed.is_char_boundary(end) {
        end -= 1;
    }
    Some(trimmed[..end].to_owned())
}

/// A declared content type kept as metadata: 1 to 255 printable ASCII bytes.
fn display_content_type(value: &str) -> Option<String> {
    (!value.is_empty()
        && value.len() <= 255
        && value
            .bytes()
            .all(|byte| byte.is_ascii_graphic() || byte == b' '))
    .then(|| value.to_owned())
}

#[cfg(test)]
#[path = "headers_tests.rs"]
mod tests;
