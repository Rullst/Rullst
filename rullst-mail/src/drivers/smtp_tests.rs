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
    // A real host without credentials is an unauthenticated relay, never a mock.
    let anonymous = SmtpDriver::try_new("smtp.example.com", 587, None, None).unwrap();
    assert_eq!(anonymous.delivery_mode(), DeliveryMode::Real);
    let blank = SmtpDriver::try_new(
        "smtp.example.com",
        587,
        Some(String::new()),
        Some(" ".to_string()),
    )
    .unwrap();
    assert_eq!(blank.delivery_mode(), DeliveryMode::Real);
    // Partial credentials are rejected instead of silently selecting the mock.
    for (username, password) in [
        (Some("user"), None),
        (None, Some("fixture-password")),
        (Some("user"), Some("")),
    ] {
        assert!(matches!(
            SmtpDriver::try_new(
                "smtp.example.com",
                587,
                username.map(str::to_string),
                password.map(str::to_string),
            ),
            Err(MailError::ConfigError(_))
        ));
    }
    let explicit_mock_user =
        SmtpDriver::try_new("smtp.example.com", 587, Some("mock_user".to_string()), None).unwrap();
    assert_eq!(
        explicit_mock_user.delivery_mode(),
        DeliveryMode::OfflineMock
    );
    let mock_host_partial =
        SmtpDriver::try_new("mock_smtp", 587, Some("user".to_string()), None).unwrap();
    assert_eq!(mock_host_partial.delivery_mode(), DeliveryMode::OfflineMock);
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

#[tokio::test]
async fn unauthenticated_or_partial_real_hosts_never_report_offline_success() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let subject = "SMTP anonymous relay contract";
    let anonymous = SmtpDriver::try_new("127.0.0.1", port, None, None).unwrap();
    assert!(matches!(
        anonymous
            .send(&base_message().subject(subject).text("body"))
            .await,
        Err(MailError::TransportError {
            provider: "smtp",
            ..
        })
    ));

    // Drivers built without try_new are checked again before sending.
    let partial = SmtpDriver {
        host: "127.0.0.1".to_string(),
        port,
        username: Some("user".to_string()),
        password: None,
    };
    assert!(matches!(
        partial
            .send(&base_message().subject(subject).text("body"))
            .await,
        Err(MailError::ConfigError(_))
    ));
    assert!(
        !OfflineMailMock::deliveries()
            .unwrap()
            .iter()
            .any(|delivery| delivery.message.subject == subject)
    );
}

#[test]
fn port_465_uses_implicit_tls_and_every_other_port_starttls() {
    assert_eq!(smtp_tls_for_port(465), SmtpTls::Implicit);
    for port in [25, 587, 2525, 1025] {
        assert_eq!(smtp_tls_for_port(port), SmtpTls::StartTls);
    }
}

/// Plaintext SMTP fixture: greets, advertises STARTTLS, records the client's
/// first two commands and refuses the upgrade.
fn spawn_starttls_refusing_server() -> (u16, std::thread::JoinHandle<Vec<String>>) {
    use std::io::{BufRead, BufReader, Write};
    use std::time::{Duration, Instant};

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(10);
        let stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline, "fixture accept timed out");
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("fixture accept failed: {error}"),
            }
        };
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut writer = stream.try_clone().unwrap();
        let mut reader = BufReader::new(stream);
        let mut commands = Vec::new();
        writer.write_all(b"220 fixture ESMTP\r\n").unwrap();
        for reply in [
            b"250-fixture\r\n250 STARTTLS\r\n".as_slice(),
            b"454 4.7.0 TLS not available\r\n",
        ] {
            let mut line = Vec::new();
            if reader.read_until(b'\n', &mut line).is_err() || line.is_empty() {
                break;
            }
            commands.push(String::from_utf8_lossy(&line).trim_end().to_string());
            if writer.write_all(reply).is_err() {
                break;
            }
        }
        commands
    });
    (port, handle)
}

#[tokio::test]
async fn submission_ports_negotiate_starttls_instead_of_implicit_tls() {
    let (port, server) = spawn_starttls_refusing_server();
    let driver = SmtpDriver::try_new(
        "127.0.0.1",
        port,
        Some("user".to_string()),
        Some("fixture-password".to_string()),
    )
    .unwrap();
    let outcome = driver.send(&base_message().text("body")).await;
    let commands = server.join().unwrap();
    assert_eq!(commands.len(), 2, "unexpected SMTP dialogue: {commands:?}");
    assert!(commands[0].starts_with("EHLO "));
    assert_eq!(commands[1], "STARTTLS");
    // The refused upgrade never falls back to plaintext AUTH or DATA.
    assert!(matches!(
        outcome,
        Err(MailError::TransportError {
            provider: "smtp",
            ..
        })
    ));
}

#[test]
fn message_builder_covers_text_html_unsubscribe_and_attachment_shapes() {
    let anonymous = Message::new()
        .to("recipient@example.com")
        .subject("text")
        .text("plain");
    assert!(matches!(
        build_smtp_message(&anonymous),
        Err(MailError::ConfigError(_))
    ));
    let text = formatted(&anonymous.from("sender@example.com"));
    assert!(text.contains("From: sender@example.com"));
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
        build_smtp_message(
            &Message::new()
                .to("not an address")
                .from("sender@example.com")
                .subject("bad")
                .text("body")
        ),
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
