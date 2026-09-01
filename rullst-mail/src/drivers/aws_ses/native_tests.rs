#![allow(clippy::unwrap_used, clippy::expect_used)]

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
