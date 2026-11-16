//! Configuration, the offline mode and the mapping to Postmark's fields.

use rullst_mail::{DeliveryMode, MailDriver, MailError, Message};
use rullst_mail_postmark_example::{PostmarkDriver, accepted, request_body};
use serde_json::{Value, json};

fn welcome() -> Message {
    Message::new()
        .to("ada@example.com")
        .from("Acme <hello@acme.example>")
        .subject("Welcome to Acme")
        .text("Thanks for signing up.")
}

fn body(message: &Message) -> Value {
    serde_json::from_slice(&request_body(message).unwrap()).unwrap()
}

#[test]
fn the_token_selects_the_offline_mode() {
    for token in ["", "  ", "mock_postmark", "MOCK_Postmark"] {
        let driver = PostmarkDriver::try_new(token).unwrap();
        assert_eq!(
            driver.delivery_mode(),
            DeliveryMode::OfflineMock,
            "{token:?}"
        );
    }
    let live = PostmarkDriver::try_new("server-token").unwrap();
    assert_eq!(live.delivery_mode(), DeliveryMode::Real);
}

#[test]
fn malformed_tokens_and_endpoints_are_configuration_errors() {
    for token in ["line\nbreak".to_string(), "x".repeat(4097)] {
        assert!(matches!(
            PostmarkDriver::try_new(token),
            Err(MailError::ConfigError(_))
        ));
    }
    assert!(PostmarkDriver::try_new("x".repeat(4096)).is_ok());
    for endpoint in [
        "http://api.postmarkapp.com/email",
        "ftp://127.0.0.1:1/email",
    ] {
        let driver = PostmarkDriver::try_new("server-token").unwrap();
        assert!(matches!(
            driver.try_with_endpoint(endpoint),
            Err(MailError::ConfigError(_))
        ));
    }
    for endpoint in [
        "https://api.postmarkapp.com/email",
        "http://127.0.0.1:9/email",
    ] {
        let driver = PostmarkDriver::try_new("server-token").unwrap();
        assert!(driver.try_with_endpoint(endpoint).is_ok(), "{endpoint}");
    }
}

#[tokio::test]
async fn the_offline_mode_captures_what_postmark_would_accept() {
    let driver = PostmarkDriver::try_new("mock_postmark").unwrap();
    let outbox = driver.offline_outbox();
    driver.send(&welcome()).await.unwrap();
    let anonymous = Message::new()
        .to("ada@example.com")
        .subject("No sender")
        .text("body");
    assert!(matches!(
        driver.send(&anonymous).await,
        Err(MailError::ConfigError(_))
    ));
    let later = welcome().send_at(chrono::Utc::now() + chrono::TimeDelta::hours(1));
    assert!(matches!(
        driver.send(&later).await,
        Err(MailError::ConfigError(_))
    ));
    let captured = outbox.lock().unwrap().clone();
    assert_eq!(captured, vec![welcome()]);
}

#[test]
fn reply_to_attachments_and_unsubscribe_headers_map_to_postmark_fields() {
    let message = welcome()
        .html(r#"<p>Hi</p><img src="cid:logo">"#)
        .reply_to("Support <support@acme.example>")
        .attach_bytes("notes.txt", b"hi".to_vec(), "text/plain")
        .attach_cid("logo", "logo.png", b"png".to_vec(), "image/png")
        .unsubscribe_url("HTTPS://acme.example/unsubscribe");
    let body = body(&message);
    assert_eq!(body["From"], "Acme <hello@acme.example>");
    assert_eq!(body["To"], "ada@example.com");
    assert_eq!(body["Subject"], "Welcome to Acme");
    assert_eq!(body["TextBody"], "Thanks for signing up.");
    assert_eq!(body["HtmlBody"], r#"<p>Hi</p><img src="cid:logo">"#);
    assert_eq!(body["ReplyTo"], "Support <support@acme.example>");
    assert_eq!(
        body["Headers"],
        json!([
            { "Name": "List-Unsubscribe", "Value": "<HTTPS://acme.example/unsubscribe>" },
            { "Name": "List-Unsubscribe-Post", "Value": "List-Unsubscribe=One-Click" },
        ])
    );
    assert_eq!(
        body["Attachments"],
        json!([
            { "Name": "notes.txt", "Content": "aGk=", "ContentType": "text/plain" },
            { "Name": "logo.png", "Content": "cG5n", "ContentType": "image/png", "ContentID": "cid:logo" },
        ])
    );
}

#[test]
fn absent_fields_are_left_out() {
    let body = body(&welcome());
    for field in ["HtmlBody", "ReplyTo", "Headers", "Attachments"] {
        assert!(body.get(field).is_none(), "{field}");
    }
    let html_only = Message::new()
        .to("ada@example.com")
        .from("hello@acme.example")
        .html("<p>Hi</p>");
    let html_body = self::body(&html_only.clone().text(""));
    assert_eq!(html_body["HtmlBody"], "<p>Hi</p>");
    assert!(html_body.get("TextBody").is_none());
    // One-click unsubscription needs HTTPS; the plain header stays.
    let plain = self::body(&welcome().unsubscribe_url("http://acme.example/unsubscribe"));
    assert_eq!(
        plain["Headers"],
        json!([{ "Name": "List-Unsubscribe", "Value": "<http://acme.example/unsubscribe>" }])
    );
    // A schedule in the past is sent now, without a field.
    let past = self::body(&welcome().send_at(chrono::Utc::now() - chrono::TimeDelta::minutes(1)));
    assert_eq!(past.as_object().map(|fields| fields.len()), Some(4));
}

#[test]
fn what_postmark_cannot_express_is_an_error() {
    let mut no_sender = welcome();
    no_sender.from = None;
    assert!(matches!(
        request_body(&no_sender),
        Err(MailError::ConfigError(_))
    ));
    let mut no_body = welcome().html("");
    no_body.body_text = None;
    assert!(matches!(
        request_body(&no_body),
        Err(MailError::ValidationError(_))
    ));
    assert!(matches!(
        request_body(&welcome().text("")),
        Err(MailError::ValidationError(_))
    ));
    let later = welcome().send_at(chrono::Utc::now() + chrono::TimeDelta::days(1));
    assert!(matches!(
        request_body(&later),
        Err(MailError::ConfigError(_))
    ));
}

#[test]
fn requests_are_bounded_at_ten_megabytes() {
    // Each ASCII text byte adds one request byte.
    let empty_text = request_body(&welcome().text("x")).unwrap().len() - 1;
    let fits = welcome().text("x".repeat(10_000_000 - empty_text));
    assert_eq!(request_body(&fits).unwrap().len(), 10_000_000);
    let over = welcome().text("x".repeat(10_000_001 - empty_text));
    assert!(matches!(
        request_body(&over),
        Err(MailError::ValidationError(_))
    ));
}

#[test]
fn receipts_need_error_code_zero_and_a_bounded_message_id() {
    let receipt = |code: Value, id: Value| json!({ "ErrorCode": code, "MessageID": id });
    let uuid = json!("0a129aee-e1cd-480d-b08d-4f48548ff48d");
    assert!(accepted(&receipt(json!(0), uuid.clone())));
    assert!(accepted(&receipt(json!(0), json!("a".repeat(128)))));
    assert!(!accepted(&receipt(json!(0), json!("a".repeat(129)))));
    assert!(!accepted(&receipt(json!(406), uuid.clone())));
    assert!(!accepted(&json!({ "MessageID": uuid })));
    for id in [
        json!(""),
        json!("id/1"),
        json!("id_1"),
        json!(12),
        Value::Null,
    ] {
        assert!(!accepted(&receipt(json!(0), id.clone())), "{id}");
    }
}
