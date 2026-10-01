#![allow(clippy::expect_used)]

use super::{MAX_PART_HEADER_BYTES, MAX_PARTS, Malformed, needs_masking, split};

const BYTERANGES: &str = "multipart/byteranges; boundary=THIS_SEPARATES";

/// A body with one part per `(headers, content)`, using `line_break`.
fn body(parts: &[(&str, &str)], line_break: &str) -> String {
    let mut body = String::new();
    for (headers, content) in parts {
        body.push_str(&format!("--THIS_SEPARATES{line_break}"));
        for header in headers.split('|').filter(|header| !header.is_empty()) {
            body.push_str(&format!("{header}{line_break}"));
        }
        body.push_str(&format!("{line_break}{content}{line_break}"));
    }
    body.push_str(&format!("--THIS_SEPARATES--{line_break}"));
    body
}

#[test]
fn parts_are_split_with_crlf_or_bare_lf_line_breaks() {
    for line_break in ["\r\n", "\n"] {
        let text = body(
            &[
                (
                    "Content-Type: text/plain|Content-Range: bytes 0-4/90",
                    "hello",
                ),
                (
                    "content-type: application/octet-stream|Content-Range: bytes 80-89/90",
                    "tail\r\n\r\nend",
                ),
            ],
            line_break,
        );
        let split = split(BYTERANGES, text.as_bytes()).expect("well-formed body");
        assert_eq!(split.parts.len(), 2);
        assert_eq!(split.parts[0].content_type, Some("text/plain"));
        assert_eq!(split.parts[0].body, b"hello");
        assert_eq!(
            split.parts[1].content_type,
            Some("application/octet-stream")
        );
        // Line breaks inside a range belong to it; only the one before the
        // delimiter is removed.
        assert_eq!(split.parts[1].body, b"tail\r\n\r\nend");
        assert!(split.parts.iter().all(|part| part.identity_encoded));
        assert_eq!(split.outside, [&b""[..], &b""[..]]);
    }
}

#[test]
fn quoted_boundaries_preamble_padding_and_epilogue_are_read() {
    let text =
        "preamble\r\n--a b'c  \r\nContent-Encoding: gzip\r\n\r\nzz\r\n--a b'c-- \t\r\nepilogue";
    let split = split("Multipart/ByteRanges; boundary=\"a b'c\"", text.as_bytes())
        .expect("well-formed body");
    assert_eq!(split.parts.len(), 1);
    assert_eq!(split.parts[0].content_type, None);
    assert!(!split.parts[0].identity_encoded);
    assert_eq!(split.parts[0].body, b"zz");
    assert_eq!(split.outside, [&b"preamble\r\n"[..], &b"epilogue"[..]]);
}

#[test]
fn bodies_that_cannot_be_split_exactly_are_malformed() {
    let oversized_header = format!("X-Pad: {}", "a".repeat(MAX_PART_HEADER_BYTES));
    let mut too_many = String::new();
    for _ in 0..=MAX_PARTS {
        too_many.push_str("--THIS_SEPARATES\r\n\r\nx\r\n");
    }
    too_many.push_str("--THIS_SEPARATES--\r\n");
    let cases = [
        ("multipart/byteranges", body(&[("", "x")], "\r\n")),
        (
            "multipart/byteranges; boundary=a; boundary=b",
            body(&[("", "x")], "\r\n"),
        ),
        (
            "multipart/byteranges; boundary=bad\"quote",
            body(&[("", "x")], "\r\n"),
        ),
        (BYTERANGES, String::new()),
        (BYTERANGES, "no delimiter at all".to_string()),
        (BYTERANGES, "--THIS_SEPARATES--\r\n".to_string()),
        (
            BYTERANGES,
            "--THIS_SEPARATES\r\n\r\nunterminated".to_string(),
        ),
        (
            BYTERANGES,
            "--THIS_SEPARATES\r\nContent-Type: text/plain".to_string(),
        ),
        (BYTERANGES, body(&[("", "--THIS_SEPARATESX")], "\r\n")),
        (BYTERANGES, body(&[("", "--THIS_SEPARATES--x")], "\r\n")),
        (
            BYTERANGES,
            body(&[("Content-Type text/plain", "x")], "\r\n"),
        ),
        (BYTERANGES, body(&[(" folded: value", "x")], "\r\n")),
        (
            BYTERANGES,
            body(&[("Content-Type : text/plain", "x")], "\r\n"),
        ),
        (BYTERANGES, body(&[("Content-Type:", "x")], "\r\n")),
        (BYTERANGES, body(&[("Content-Encoding:", "x")], "\r\n")),
        (
            BYTERANGES,
            body(
                &[("Content-Type: text/plain|content-type: text/html", "x")],
                "\r\n",
            ),
        ),
        (BYTERANGES, body(&[(&oversized_header, "x")], "\r\n")),
        (BYTERANGES, too_many),
    ];
    for (content_type, text) in &cases {
        assert_eq!(
            split(content_type, text.as_bytes()).err(),
            Some(Malformed),
            "{content_type}: {text:.80}"
        );
    }

    let mut most = String::new();
    for _ in 0..MAX_PARTS {
        most.push_str("--THIS_SEPARATES\r\n\r\nx\r\n");
    }
    most.push_str("--THIS_SEPARATES--\r\n");
    assert_eq!(
        split(BYTERANGES, most.as_bytes())
            .expect("the part limit is inclusive")
            .parts
            .len(),
        MAX_PARTS
    );
}

#[test]
fn only_textual_identity_encoded_ranges_are_checked() {
    let secret = "key AKIAIOSFODNN7EXAMPLE";
    for (headers, content, expected) in [
        ("Content-Type: text/plain; charset=utf-8", secret, true),
        ("Content-Type: text/html", secret, true),
        ("", secret, true),
        (
            "Content-Type: application/json",
            r#"{"db":"postgres://app:hunter2@db/app"}"#,
            true,
        ),
        (
            "Content-Type: application/yaml",
            "db: postgres://app:hunter2@db/app",
            true,
        ),
        (
            "Content-Type: application/json",
            r#"{"ts":1727712000000}"#,
            false,
        ),
        ("Content-Type: application/octet-stream", secret, false),
        ("Content-Type: image/png", secret, false),
        ("Content-Type: text/event-stream", secret, false),
        (
            "Content-Type: text/plain|Content-Encoding: gzip",
            secret,
            false,
        ),
        (
            "Content-Type: text/plain|Content-Encoding: identity",
            secret,
            true,
        ),
        ("Content-Type: text/plain", "an ordinary range", false),
    ] {
        let text = body(
            &[("Content-Type: text/plain", "clean"), (headers, content)],
            "\r\n",
        );
        assert_eq!(
            needs_masking(BYTERANGES, text.as_bytes()),
            Ok(expected),
            "{headers}"
        );
    }

    // Bytes outside every part are checked as text.
    let clean = body(&[("Content-Type: text/plain", "clean")], "\r\n");
    assert_eq!(
        needs_masking(BYTERANGES, format!("{secret}\r\n{clean}").as_bytes()),
        Ok(true)
    );
    assert_eq!(
        needs_masking(BYTERANGES, format!("{clean}{secret}").as_bytes()),
        Ok(true)
    );

    // A range that is not UTF-8 is not rewritten, as a whole response is not.
    let mut binary = b"--THIS_SEPARATES\r\nContent-Type: text/plain\r\n\r\n\xff".to_vec();
    binary.extend_from_slice(secret.as_bytes());
    binary.extend_from_slice(b"\r\n--THIS_SEPARATES--\r\n");
    assert_eq!(needs_masking(BYTERANGES, &binary), Ok(false));
}

#[test]
fn splitting_is_linear_in_the_body_length() {
    // Many short lines, and lines that almost match the delimiter.
    let content = "--THIS_SEPARATE\n".repeat(100_000);
    let text = body(&[("Content-Type: text/plain", &content)], "\r\n");
    let started = std::time::Instant::now();
    assert_eq!(needs_masking(BYTERANGES, text.as_bytes()), Ok(false));
    assert!(
        started.elapsed() < std::time::Duration::from_secs(5),
        "{:?}",
        started.elapsed()
    );
}
