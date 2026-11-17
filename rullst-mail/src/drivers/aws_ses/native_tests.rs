#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::super::limits::{MAX_SES_V2_MESSAGE_BYTES, add_size, message_size_error};
use super::*;
use crate::attachment::Attachment;

fn base_message() -> Message {
    Message::new()
        .to("recipient@example.com")
        .from("sender@example.com")
        .subject("native contract")
        .html("<p>body</p>")
        .text("body")
        .unsubscribe_url("https://example.com/unsubscribe")
}

#[test]
fn native_content_builds_headers_and_both_attachment_dispositions() {
    let message = base_message()
        .attach(Attachment::new(
            "report.txt",
            b"report".to_vec(),
            "text/plain",
        ))
        .attach(Attachment::inline(
            "logo",
            "logo.png",
            b"png".to_vec(),
            "image/png",
        ));
    validate_message_limits(&message).unwrap();
    assert!(build_content(&message).is_ok());
    assert!(header("X-Contract", "value".to_string()).is_ok());
}

#[test]
fn native_headers_attachments_and_size_accounting_fail_closed() {
    for (name, value) in [
        ("", "value".to_string()),
        ("Bad:Name", "value".to_string()),
        ("X-Test", String::new()),
        ("X-Test", "line\nbreak".to_string()),
        ("X-Test", "x".repeat(996)),
    ] {
        assert!(header(name, value).is_err());
    }
    // A name holds at most 126 bytes, a value at most 995, and both together
    // at most 996, which with the `: ` separator fill a 998-byte line.
    let long_name = "N".repeat(126);
    assert!(header(&long_name, "v".to_string()).is_ok());
    assert!(header(&"N".repeat(127), "v".to_string()).is_err());
    assert!(header("X", "x".repeat(995)).is_ok());
    assert!(header("X-Test", "x".repeat(990)).is_ok());
    assert!(header("X-Test", "x".repeat(991)).is_err());

    for attachment in [
        Attachment::new("", vec![1], "text/plain"),
        Attachment::new("x", vec![1], "x".repeat(79)),
        Attachment::inline("x".repeat(79), "x", vec![1], "text/plain"),
    ] {
        let message = base_message().attach(attachment);
        assert!(validate_message_limits(&message).is_err());
    }

    let mut total = MAX_SES_V2_MESSAGE_BYTES;
    assert!(add_size(&mut total, 1).is_err());
    let mut overflow = usize::MAX;
    assert!(add_size(&mut overflow, 1).is_err());
    assert!(matches!(
        message_size_error(),
        MailError::ValidationError(_)
    ));
}

#[test]
fn ses_field_and_encoded_size_bounds_are_inclusive() {
    // 40 MiB fits exactly; one byte more does not.
    let mut total = 0;
    assert!(add_size(&mut total, 41_943_040).is_ok());
    assert!(add_size(&mut total, 1).is_err());

    for attachment in [
        Attachment::new("f".repeat(255), vec![1], "text/plain"),
        Attachment::new("f", vec![1], "m".repeat(78)),
        Attachment::inline("c".repeat(78), "f", vec![1], "text/plain"),
    ] {
        let message = base_message().attach(attachment);
        assert!(validate_message_limits(&message).is_ok());
    }
    let long_name = base_message().attach(Attachment::new("f".repeat(256), vec![1], "text/plain"));
    assert!(validate_message_limits(&long_name).is_err());

    // The estimate is a 4,096-byte envelope allowance plus each JSON string:
    // `"a@b.co"` (8), `""` (2), `"f"` (3) and `"a/b"` (5), 4,114 bytes, and
    // 31,454,199 content bytes encode to 41,938,932 base64 bytes. That is 6
    // bytes over 40 MiB.
    let mut message = Message::new().to("a@b.co");
    message
        .attachments
        .push(Attachment::new("f", vec![0_u8; 31_454_199], "a/b"));
    assert!(validate_message_limits(&message).is_err());
    // Six content bytes fewer remove 8 base64 bytes and fit.
    message.attachments[0].content.truncate(31_454_193);
    assert!(validate_message_limits(&message).is_ok());
}
