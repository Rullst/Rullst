//! Bounded `_token` lookup in `multipart/form-data` request bodies.
//!
//! A server-rendered upload form cannot set the `X-CSRF-Token` header, so its
//! token arrives as a `_token` part. Only the start of the body is read: the
//! `_token` part must end within [`MAX_TOKEN_PREFIX_BYTES`], so it must come
//! before any file part. Every byte read is replayed in front of the rest of
//! the body, which is never buffered as a whole.

use axum::body::{Body, Bytes};
use http_body::{Body as HttpBody, Frame, SizeHint};
use std::collections::VecDeque;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Longest multipart body prefix read while looking for `_token`.
pub(super) const MAX_TOKEN_PREFIX_BYTES: usize = 64 * 1024;
/// Longest accepted `_token` value, matching the CSRF cookie bound.
const MAX_TOKEN_BYTES: usize = 128;

/// Returns the boundary of a `multipart/form-data` content type.
///
/// The boundary must satisfy RFC 2046: 1-70 characters from its `bchars`
/// set, not ending in a space. A malformed or repeated parameter fails.
pub(super) fn form_data_boundary(content_type: &str) -> Option<&str> {
    let mut parameters = content_type.split(';');
    if !parameters
        .next()?
        .trim()
        .eq_ignore_ascii_case("multipart/form-data")
    {
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
        boundary = Some(unquote(value.trim()));
    }
    let boundary = boundary?;
    let valid = (1..=70).contains(&boundary.len())
        && !boundary.ends_with(' ')
        && boundary
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"'()+_,-./:=? ".contains(&byte));
    valid.then_some(boundary)
}

fn unquote(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|quoted| quoted.strip_suffix('"'))
        .unwrap_or(value)
}

/// Outcome of scanning the start of a multipart body.
#[derive(Debug, PartialEq, Eq)]
enum Scan {
    /// Value of the first `_token` field.
    Token(Vec<u8>),
    /// The bytes read so far end before the first `_token` field does.
    Incomplete,
    /// The body cannot provide a usable `_token` field.
    Missing,
}

/// Scans complete parts from the start of `prefix` for the first `_token`
/// field. Browsers send no preamble, so the body must start with the first
/// delimiter; anything malformed is `Missing`.
fn scan(prefix: &[u8], boundary: &[u8]) -> Scan {
    let mut delimiter = Vec::with_capacity(boundary.len() + 4);
    delimiter.extend_from_slice(b"\r\n--");
    delimiter.extend_from_slice(boundary);
    let first = &delimiter[2..];
    let available = prefix.len().min(first.len());
    if prefix[..available] != first[..available] {
        return Scan::Missing;
    }
    if available < first.len() {
        return Scan::Incomplete;
    }
    let mut position = first.len();
    loop {
        // After a delimiter, CRLF starts another part and `--` ends the body.
        match prefix.get(position..position + 2) {
            None => return Scan::Incomplete,
            Some(b"\r\n") => position += 2,
            Some(_) => return Scan::Missing,
        }
        let headers = if prefix[position..].starts_with(b"\r\n") {
            position += 2;
            &[][..]
        } else {
            let Some(length) = find(&prefix[position..], b"\r\n\r\n") else {
                return Scan::Incomplete;
            };
            let headers = &prefix[position..position + length];
            position += length + 4;
            headers
        };
        let Some(length) = find(&prefix[position..], &delimiter) else {
            return Scan::Incomplete;
        };
        let value = &prefix[position..position + length];
        position += length + delimiter.len();
        if is_token_field(headers) {
            return if (1..=MAX_TOKEN_BYTES).contains(&value.len()) {
                Scan::Token(value.to_vec())
            } else {
                Scan::Missing
            };
        }
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Whether a part's headers describe the plain `_token` form field: exactly
/// one `Content-Disposition: form-data; name="_token"` without a file name.
fn is_token_field(headers: &[u8]) -> bool {
    let Ok(headers) = std::str::from_utf8(headers) else {
        return false;
    };
    let mut disposition = None;
    for line in headers.split("\r\n") {
        let Some((name, value)) = line.split_once(':') else {
            return false;
        };
        if name.trim().eq_ignore_ascii_case("content-disposition") {
            if disposition.is_some() {
                return false;
            }
            disposition = Some(value);
        }
    }
    let Some(disposition) = disposition else {
        return false;
    };
    let mut parameters = disposition.split(';');
    if !parameters
        .next()
        .is_some_and(|kind| kind.trim().eq_ignore_ascii_case("form-data"))
    {
        return false;
    }
    let mut is_token = false;
    for parameter in parameters {
        let Some((name, value)) = parameter.split_once('=') else {
            return false;
        };
        let name = name.trim();
        if name.eq_ignore_ascii_case("filename") || name.eq_ignore_ascii_case("filename*") {
            return false;
        }
        if name.eq_ignore_ascii_case("name") {
            is_token = unquote(value.trim()) == "_token";
        }
    }
    is_token
}

/// The request body could not be read.
#[derive(Debug)]
pub(super) struct BodyReadError;

/// Reads the start of a multipart body until its first `_token` field is
/// complete, the body ends or [`MAX_TOKEN_PREFIX_BYTES`] have been read.
///
/// Returns the token, if one was found, and a body that replays every frame
/// read before the rest of the original body. The prefix is rescanned only
/// after it has doubled, so a body arriving in tiny frames costs linear time.
pub(super) async fn read_token(
    mut body: Body,
    boundary: &str,
) -> Result<(Option<Vec<u8>>, Body), BodyReadError> {
    let mut frames = VecDeque::new();
    let mut prefix = Vec::new();
    let mut scanned = 0_usize;
    let token = loop {
        let ended = match std::future::poll_fn(|cx| Pin::new(&mut body).poll_frame(cx)).await {
            None => true,
            Some(Err(_)) => return Err(BodyReadError),
            Some(Ok(frame)) => {
                if let Some(data) = frame.data_ref() {
                    let room = MAX_TOKEN_PREFIX_BYTES.saturating_sub(prefix.len());
                    prefix.extend_from_slice(&data[..data.len().min(room)]);
                }
                // A trailers frame is the last frame of a body.
                let trailers = !frame.is_data();
                frames.push_back(frame);
                trailers
            }
        };
        let full = prefix.len() >= MAX_TOKEN_PREFIX_BYTES;
        if !ended && !full && prefix.len() < scanned.saturating_mul(2).max(1) {
            continue;
        }
        scanned = prefix.len();
        match scan(&prefix, boundary.as_bytes()) {
            Scan::Token(token) => break Some(token),
            Scan::Incomplete if !ended && !full => {}
            Scan::Incomplete | Scan::Missing => break None,
        }
    };
    Ok((
        token,
        Body::new(ReplayBody {
            frames,
            inner: body,
        }),
    ))
}

/// Replays the frames read while looking for the token, then the rest of
/// the original body.
struct ReplayBody {
    frames: VecDeque<Frame<Bytes>>,
    inner: Body,
}

impl HttpBody for ReplayBody {
    type Data = Bytes;
    type Error = axum::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, axum::Error>>> {
        if let Some(frame) = self.frames.pop_front() {
            return Poll::Ready(Some(Ok(frame)));
        }
        Pin::new(&mut self.inner).poll_frame(cx)
    }

    fn is_end_stream(&self) -> bool {
        self.frames.is_empty() && self.inner.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        let buffered = self
            .frames
            .iter()
            .filter_map(Frame::data_ref)
            .map(|data| u64::try_from(data.len()).unwrap_or(u64::MAX))
            .fold(0_u64, u64::saturating_add);
        let inner = self.inner.size_hint();
        let mut hint = SizeHint::new();
        hint.set_lower(inner.lower().saturating_add(buffered));
        if let Some(upper) = inner.upper() {
            hint.set_upper(upper.saturating_add(buffered));
        }
        hint
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn part(name: &str, value: &str) -> String {
        format!("--XyZ\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n")
    }

    #[test]
    fn boundaries_follow_rfc_2046() {
        for (content_type, expected) in [
            ("multipart/form-data; boundary=XyZ", Some("XyZ")),
            ("Multipart/Form-Data;boundary=\"a b\";", Some("a b")),
            (
                "multipart/form-data; charset=utf-8; BOUNDARY=x-1",
                Some("x-1"),
            ),
            ("multipart/form-data", None),
            ("multipart/form-data; boundary=", None),
            ("multipart/form-data; boundary=\"trailing \"", None),
            ("multipart/form-data; boundary=a; boundary=b", None),
            ("multipart/form-data; boundary=semi;colon", None),
            ("multipart/mixed; boundary=XyZ", None),
            ("application/x-www-form-urlencoded", None),
        ] {
            assert_eq!(form_data_boundary(content_type), expected, "{content_type}");
        }
        assert_eq!(
            form_data_boundary(&format!("multipart/form-data; boundary={}", "b".repeat(71))),
            None
        );
    }

    #[test]
    fn the_first_token_field_is_found_after_other_fields() {
        let body = format!("{}{}--XyZ--\r\n", part("title", "x"), part("_token", "abc"));
        assert_eq!(scan(body.as_bytes(), b"XyZ"), Scan::Token(b"abc".to_vec()));
        // The token part is complete as soon as the next delimiter arrives.
        let streaming = format!("{}--XyZ\r\nContent-Disp", part("_token", "abc"));
        assert_eq!(
            scan(streaming.as_bytes(), b"XyZ"),
            Scan::Token(b"abc".to_vec())
        );
    }

    #[test]
    fn incomplete_malformed_and_file_parts_are_not_tokens() {
        let token = part("_token", "abc");
        assert_eq!(scan(&token.as_bytes()[..3], b"XyZ"), Scan::Incomplete);
        assert_eq!(scan(token.as_bytes(), b"XyZ"), Scan::Incomplete);
        assert_eq!(scan(b"preamble\r\n--XyZ\r\n", b"XyZ"), Scan::Missing);
        assert_eq!(scan(b"--XyZ--\r\n", b"XyZ"), Scan::Missing);
        let file = "--XyZ\r\nContent-Disposition: form-data; name=\"_token\"; filename=\"t.txt\"\r\n\r\nabc\r\n--XyZ--";
        assert_eq!(scan(file.as_bytes(), b"XyZ"), Scan::Missing);
        let empty = format!("{}--XyZ--", part("_token", ""));
        assert_eq!(scan(empty.as_bytes(), b"XyZ"), Scan::Missing);
        let oversized = format!("{}--XyZ--", part("_token", &"t".repeat(129)));
        assert_eq!(scan(oversized.as_bytes(), b"XyZ"), Scan::Missing);
        let duplicate_disposition = "--XyZ\r\nContent-Disposition: form-data; name=\"_token\"\r\ncontent-disposition: form-data; name=\"x\"\r\n\r\nabc\r\n--XyZ--";
        assert_eq!(
            scan(duplicate_disposition.as_bytes(), b"XyZ"),
            Scan::Missing
        );
    }
}
