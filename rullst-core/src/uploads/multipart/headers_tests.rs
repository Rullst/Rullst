#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;

fn parsed(raw: &str) -> Result<PartHeaders, MultipartError> {
    parse(raw.as_bytes(), 128)
}

fn file(raw: &str) -> FileMeta {
    parsed(raw)
        .expect("valid file part")
        .file
        .expect("a file part")
}

#[test]
fn a_text_field_has_a_name_and_no_file() {
    assert_eq!(
        parsed("Content-Disposition: form-data; name=\"title\"").unwrap(),
        PartHeaders {
            name: "title".to_string(),
            file: None
        }
    );
    // Names, values and the disposition type ignore ASCII case and spacing;
    // unknown headers and parameters are ignored.
    assert_eq!(
        parsed("X-Trace: 1\r\nCONTENT-DISPOSITION:FORM-DATA ;  NAME = plain ; size=3 ;\r\ncontent-type: text/plain")
            .unwrap(),
        PartHeaders {
            name: "plain".to_string(),
            file: None
        }
    );
}

#[test]
fn a_file_part_keeps_display_metadata_only() {
    assert_eq!(
        file(
            "Content-Disposition: form-data; name=\"doc\"; filename=\"report.pdf\"\r\nContent-Type: application/pdf"
        ),
        FileMeta {
            display_name: Some("report.pdf".to_string()),
            content_type: Some("application/pdf".to_string()),
            empty_file_name: false,
        }
    );
    // A browser's empty file input.
    assert_eq!(
        file(
            "Content-Disposition: form-data; name=\"doc\"; filename=\"\"\r\nContent-Type: application/octet-stream"
        ),
        FileMeta {
            display_name: None,
            content_type: Some("application/octet-stream".to_string()),
            empty_file_name: true,
        }
    );
    // Only an RFC 5987 name: a file without a display name.
    assert_eq!(
        file("Content-Disposition: form-data; name=\"doc\"; filename*=UTF-8''r%C3%A9sum%C3%A9.pdf"),
        FileMeta {
            display_name: None,
            content_type: None,
            empty_file_name: false,
        }
    );
    assert!(
        !file("Content-Disposition: form-data; name=\"doc\"; filename=\"\"; filename*=UTF-8''a")
            .empty_file_name
    );
}

#[test]
fn quoted_values_unescape_quotes_and_keep_windows_paths() {
    let display = |disposition: &str| {
        file(&format!(
            "Content-Disposition: form-data; name=\"doc\"; {disposition}"
        ))
        .display_name
    };
    assert_eq!(
        display(r#"filename="say \"hi\".txt""#).as_deref(),
        Some("say \"hi\".txt")
    );
    // `\\` is an escaped backslash, which still separates path segments.
    assert_eq!(
        display(r#"filename="dir\\a.txt""#).as_deref(),
        Some("a.txt")
    );
    // An old browser's full Windows path, with `;` inside the quotes.
    assert_eq!(
        display(r#"filename="C:\Users\ana\notes; v2.txt""#).as_deref(),
        Some("notes; v2.txt")
    );
    assert_eq!(
        parsed(r#"Content-Disposition: form-data; name="a\"b""#)
            .unwrap()
            .name,
        "a\"b"
    );
}

#[test]
fn display_names_drop_paths_and_invisible_characters() {
    for (raw, expected) in [
        ("../../etc/passwd", Some("passwd")),
        ("..\\..\\windows\\win.ini", Some("win.ini")),
        ("/abs/path/", None),
        ("..", None),
        ("dir/.", None),
        (" .. ", None),
        ("  spaced name.txt  ", Some("spaced name.txt")),
        ("report\u{202E}fdp.png", Some("reportfdp.png")),
        ("tab\tand\u{7}bell\u{200B}.txt", Some("tabandbell.txt")),
        ("\u{0}\u{1f}", None),
        ("emoji \u{1F600}.png", Some("emoji \u{1F600}.png")),
    ] {
        assert_eq!(display_name(raw).as_deref(), expected, "{raw:?}");
    }
    let long = "é".repeat(200);
    let cut = display_name(&long).unwrap();
    assert_eq!(cut.len(), 254);
    assert_eq!(cut, "é".repeat(127));
    assert_eq!(display_name(&"a".repeat(300)).unwrap().len(), 255);
}

#[test]
fn declared_content_types_are_kept_only_when_printable_and_short() {
    let declared = |value: &str| {
        file(&format!(
            "Content-Disposition: form-data; name=\"f\"; filename=\"a\"\r\nContent-Type: {value}"
        ))
        .content_type
    };
    assert_eq!(
        declared("image/png; charset=binary").as_deref(),
        Some("image/png; charset=binary")
    );
    assert_eq!(declared(""), None);
    assert_eq!(declared("text/plain\u{1}"), None);
    assert_eq!(declared("image/\u{e9}"), None);
    assert_eq!(
        declared(&"a".repeat(255)).map(|value| value.len()),
        Some(255)
    );
    assert_eq!(declared(&"a".repeat(256)), None);
}

#[test]
fn field_names_are_limited() {
    let named = |name: &str| {
        parse(
            format!("Content-Disposition: form-data; name=\"{name}\"").as_bytes(),
            8,
        )
    };
    assert_eq!(named("eightchr").unwrap().name, "eightchr");
    assert_eq!(named("ninechars"), Err(MultipartError::FieldNameTooLong));
}

#[test]
fn malformed_headers_fail_with_a_fixed_reason() {
    for (raw, reason) in [
        ("", "a part has no Content-Disposition"),
        (
            "Content-Type: text/plain",
            "a part has no Content-Disposition",
        ),
        (
            "Content-Disposition form-data",
            "a part header has no colon",
        ),
        (
            " Content-Disposition: form-data; name=a",
            "a part header name is not a token",
        ),
        (": form-data", "a part header name is not a token"),
        (
            "Content-Disposition: form-data; name=a\r\nContent-Disposition: form-data; name=b",
            "a part repeats a header",
        ),
        (
            "Content-Disposition: form-data; name=a\r\nContent-Type: a\r\ncontent-type: b",
            "a part repeats a header",
        ),
        (
            "Content-Disposition: attachment; name=a",
            "a part is not form-data",
        ),
        (
            "Content-Disposition: form-data; name=a\r\nContent-Type: MULTIPART/mixed; boundary=x",
            "nested multipart parts are not supported",
        ),
        ("Content-Disposition: form-data", "a part has no field name"),
        (
            "Content-Disposition: form-data; name=\"\"",
            "a part has no field name",
        ),
        (
            "Content-Disposition: form-data; name",
            "malformed Content-Disposition parameters",
        ),
        (
            "Content-Disposition: form-data; =a",
            "malformed Content-Disposition parameters",
        ),
        (
            "Content-Disposition: form-data; na me=a",
            "malformed Content-Disposition parameters",
        ),
        (
            "Content-Disposition: form-data; name=\"a\" junk",
            "malformed Content-Disposition parameters",
        ),
        (
            "Content-Disposition: form-data name=a",
            "a part is not form-data",
        ),
        (
            "Content-Disposition: form-data; name=\"a",
            "a Content-Disposition value is not closed",
        ),
        (
            "Content-Disposition: form-data; name=a; NAME=b",
            "a Content-Disposition parameter is repeated",
        ),
        (
            "Content-Disposition: form-data; name=a; filename=x; filename=y",
            "a Content-Disposition parameter is repeated",
        ),
        (
            "Content-Disposition: form-data; name=a; filename*=x; FILENAME*=y",
            "a Content-Disposition parameter is repeated",
        ),
    ] {
        assert_eq!(
            parsed(raw),
            Err(MultipartError::Malformed(reason)),
            "{raw:?}"
        );
    }
    assert_eq!(
        parse(b"Content-Disposition: form-data; name=\"\xff\"", 128),
        Err(MultipartError::Malformed("part headers are not UTF-8"))
    );
}
