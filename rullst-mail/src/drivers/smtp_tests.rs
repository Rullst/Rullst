#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use crate::attachment::Attachment;
use crate::drivers::OfflineMailMock;

fn base_message() -> Message {
    Message::new()
        .to("recipient@example.com")
        .from("sender@example.com")
        .subject("SMTP contract")
}

fn formatted(message: &Message) -> String {
    String::from_utf8_lossy(&build_smtp_message(message).unwrap().formatted()).into_owned()
}

#[test]
fn configuration_selects_real_or_offline_delivery_and_rejects_unsafe_values() {
    let empty_host = SmtpDriver::try_new("", 25, None, None).unwrap();
    assert_eq!(empty_host.delivery_mode(), DeliveryMode::OfflineMock);
    let mock_host = SmtpDriver::try_new("mock_smtp", 25, None, None).unwrap();
    assert_eq!(mock_host.delivery_mode(), DeliveryMode::OfflineMock);
    let anonymous = SmtpDriver::try_new("smtp.example.com", 587, None, None).unwrap();
    assert_eq!(anonymous.delivery_mode(), DeliveryMode::OfflineMock);
    let partial =
        SmtpDriver::try_new("smtp.example.com", 587, Some("user".to_string()), None).unwrap();
    assert_eq!(partial.delivery_mode(), DeliveryMode::OfflineMock);
    let mock_password = SmtpDriver::try_new(
        "smtp.example.com",
        587,
        Some("user".to_string()),
        Some("mock_password".to_string()),
    )
    .unwrap();
    assert_eq!(mock_password.delivery_mode(), DeliveryMode::OfflineMock);
    let real = SmtpDriver::try_new(
        "smtp.example.com",
        587,
        Some("user".to_string()),
        Some("secret".to_string()),
    )
    .unwrap();
    assert_eq!(real.delivery_mode(), DeliveryMode::Real);

    assert!(SmtpDriver::try_new("smtp.example.com", 0, None, None).is_err());
    assert!(SmtpDriver::try_new("smtp\r\n.invalid", 25, None, None).is_err());
    assert!(
        SmtpDriver::try_new("smtp.example.com", 25, Some("user\nname".to_string()), None,).is_err()
    );
    assert!(
        SmtpDriver::try_new(
            "smtp.example.com",
            25,
            Some("user".to_string()),
            Some("pass\rword".to_string()),
        )
        .is_err()
    );
}

#[tokio::test]
async fn offline_send_records_sanitized_delivery_and_real_transport_fails_typed() {
    let offline = SmtpDriver::try_new("mock_smtp", 25, None, None).unwrap();
    let offline_message = base_message()
        .to("smtp-contract@example.com")
        .subject("SMTP offline contract")
        .text("body");
    offline.send(&offline_message).await.unwrap();
    let deliveries = OfflineMailMock::deliveries().unwrap();
    assert!(deliveries.iter().any(|delivery| {
        delivery.provider == "smtp"
            && delivery.message.to == "smtp-contract@example.com"
            && delivery.message.subject == "SMTP offline contract"
    }));

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let real = SmtpDriver::try_new(
        "127.0.0.1",
        port,
        Some("user".to_string()),
        Some("secret".to_string()),
    )
    .unwrap();
    assert!(matches!(
        real.send(&base_message().text("body")).await,
        Err(MailError::TransportError {
            provider: "smtp",
            ..
        })
    ));
}

#[test]
fn message_builder_covers_text_html_unsubscribe_and_attachment_shapes() {
    let text = formatted(
        &Message::new()
            .to("recipient@example.com")
            .subject("text")
            .text("plain"),
    );
    assert!(text.contains("From: noreply@rullst.dev"));
    assert!(text.contains("Content-Type: text/plain"));

    let mut html_only = base_message().html("<strong>HTML</strong>");
    html_only.body_text = None;
    assert!(formatted(&html_only).contains("Content-Type: text/html"));

    let alternative = formatted(
        &base_message()
            .text("plain")
            .html("<strong>HTML</strong>")
            .unsubscribe_email("leave@example.com")
            .unsubscribe_url("https://example.com/leave"),
    );
    assert!(alternative.contains("multipart/alternative"));
    assert!(alternative.contains("List-Unsubscribe:"));
    assert!(alternative.contains("List-Unsubscribe-Post: List-Unsubscribe=One-Click"));

    let mut related = base_message().html("<img src=\"cid:brand\">").attach_cid(
        "brand",
        "brand.png",
        [1_u8, 2, 3],
        "image/png",
    );
    related.body_text = None;
    assert!(formatted(&related).contains("multipart/related"));

    let mixed = formatted(&base_message().text("plain").attach_bytes(
        "terms.txt",
        b"terms".to_vec(),
        "text/plain",
    ));
    assert!(mixed.contains("multipart/mixed"));
    assert!(mixed.contains("filename=\"terms.txt\""));
}

#[test]
fn message_builder_rejects_addresses_empty_body_and_invalid_attachment_mime() {
    assert!(matches!(
        build_smtp_message(&base_message().from("not an address").text("body")),
        Err(MailError::ValidationError(message)) if message.contains("invalid sender")
    ));
    assert!(matches!(
        build_smtp_message(&Message::new().to("not an address").subject("bad").text("body")),
        Err(MailError::ValidationError(message)) if message.contains("invalid recipient")
    ));
    assert!(build_smtp_message(&base_message()).is_err());

    let invalid = base_message().text("body").attach(Attachment::new(
        "payload.bin",
        [1_u8],
        "not a mime type",
    ));
    assert!(matches!(
        build_smtp_message(&invalid),
        Err(MailError::ValidationError(message)) if message.contains("MIME")
    ));
}
