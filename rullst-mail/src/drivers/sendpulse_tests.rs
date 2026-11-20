use super::*;
use crate::Attachment;

#[test]
fn native_request_preserves_reset_link_and_encodes_html_and_attachments() {
    let message = Message::new()
        .from("accounts@example.com")
        .to("member@example.com")
        .subject("Reset")
        .html("<a href=\"https://app.example/reset?token=opaque_123\">Reset</a>")
        .text("password=accidental")
        .attach(Attachment::new("note.txt", b"hello".to_vec(), "text/plain"));
    let prepared = DeliveryPipeline::prepare(&message).unwrap();
    let body = rest::encode(&payload(prepared.message()).unwrap()).unwrap();
    let request = SendPulseDriver::try_new("fixture_sendpulse")
        .unwrap()
        .request(body)
        .unwrap();
    assert_eq!(
        request.url().as_str(),
        "https://api.sendpulse.com/smtp/emails"
    );
    assert_eq!(
        request.headers()["authorization"],
        "Bearer fixture_sendpulse"
    );
    let value: Value = serde_json::from_slice(request.body().unwrap().as_bytes().unwrap()).unwrap();
    assert_eq!(value["email"]["to"][0]["email"], "member@example.com");
    let html = STANDARD
        .decode(value["email"]["html"].as_str().unwrap())
        .unwrap();
    assert!(
        String::from_utf8(html)
            .unwrap()
            .contains("token=opaque_123")
    );
    assert!(
        !value["email"]["text"]
            .as_str()
            .unwrap()
            .contains("accidental")
    );
    assert_eq!(value["email"]["attachments_binary"]["note.txt"], "aGVsbG8=");
    assert!(validate_receipt(&json!({"result":true,"id":"accepted-id"})).is_ok());
    for bad in [
        json!({"result":false,"id":"accepted-id"}),
        json!({"result":true}),
        json!({"result":true,"id":""}),
    ] {
        assert!(validate_receipt(&bad).is_err());
    }
    let duplicate =
        message
            .clone()
            .attach(Attachment::new("note.txt", b"other".to_vec(), "text/plain"));
    assert!(payload(&duplicate).is_err());
    assert!(
        payload(
            &message
                .clone()
                .unsubscribe_url("https://app.example/unsubscribe")
        )
        .is_err()
    );
}

#[tokio::test]
async fn offline_fallback_and_future_rejection_need_no_network() {
    let message = Message::new()
        .from("accounts@example.com")
        .to("member@example.com")
        .subject("Welcome")
        .text("Welcome");
    for key in ["", "mock_sendpulse"] {
        SendPulseDriver::try_new(key)
            .unwrap()
            .send(&message)
            .await
            .unwrap();
    }
    assert!(SendPulseDriver::try_new("bad\r\nkey").is_err());
    let scheduled = message.send_at(jiff::Timestamp::now() + jiff::SignedDuration::from_mins(5));
    assert!(
        SendPulseDriver::try_new("fixture_sendpulse")
            .unwrap()
            .send(&scheduled)
            .await
            .is_err()
    );
}

#[test]
fn reply_to_is_sent_as_a_mailbox_object() {
    let message = Message::new()
        .from("site@example.com")
        .to("owner@example.com")
        .subject("New booking")
        .text("A visitor asked for a class")
        .reply_to("Ana Souza <ana@example.com>");
    let value = payload(&message).unwrap();
    assert_eq!(
        value["email"]["reply_to"],
        json!({"email": "ana@example.com", "name": "Ana Souza"})
    );
    let bare = payload(&message.clone().reply_to("ana@example.com")).unwrap();
    assert_eq!(
        bare["email"]["reply_to"],
        json!({"email": "ana@example.com"})
    );
    let without = Message {
        reply_to: None,
        ..message
    };
    assert!(
        payload(&without).unwrap()["email"]
            .get("reply_to")
            .is_none()
    );
}

/// A test with a `mock_*` key refuses what live delivery refuses (inline CID,
/// unsubscribe headers, a future schedule, no sender or body, duplicate
/// attachment names), before anything is captured.
#[tokio::test]
async fn offline_mock_refuses_what_live_delivery_refuses() {
    let driver = SendPulseDriver::try_new("mock_sendpulse").unwrap();
    let message = Message::new()
        .from("accounts@example.com")
        .to("member@example.com")
        .subject("sendpulse offline refusal")
        .text("Welcome");
    let unsupported = |error: &MailError| matches!(error, MailError::ValidationError(text) if text.contains("SendPulse REST does not support"));
    let inline = message.clone().html(r#"<img src="cid:logo">"#).attach_cid(
        "logo",
        "logo.png",
        b"png".to_vec(),
        "image/png",
    );
    assert!(unsupported(&driver.send(&inline).await.unwrap_err()));
    let unsubscribe = message
        .clone()
        .unsubscribe_url("https://app.example/unsubscribe");
    assert!(unsupported(&driver.send(&unsubscribe).await.unwrap_err()));
    let scheduled = message
        .clone()
        .send_at(jiff::Timestamp::now() + jiff::SignedDuration::from_mins(5));
    assert!(matches!(
        driver.send(&scheduled).await,
        Err(MailError::ConfigError(text)) if text.contains("cannot schedule")
    ));
    let unsigned = Message {
        from: None,
        ..message.clone()
    };
    assert!(matches!(
        driver.send(&unsigned).await,
        Err(MailError::ConfigError(text)) if text.contains("verified sender")
    ));
    let empty = Message {
        body_text: None,
        ..message.clone()
    };
    assert!(matches!(
        driver.send(&empty).await,
        Err(MailError::ValidationError(text)) if text.contains("nonempty text or HTML")
    ));
    let duplicate = message
        .clone()
        .attach(Attachment::new("note.txt", b"one".to_vec(), "text/plain"))
        .attach(Attachment::new("note.txt", b"two".to_vec(), "text/plain"));
    assert!(matches!(
        driver.send(&duplicate).await,
        Err(MailError::ValidationError(text)) if text.contains("unique")
    ));
    assert!(
        !crate::drivers::OfflineMailMock::deliveries()
            .unwrap()
            .iter()
            .any(|delivery| delivery.message.subject == "sendpulse offline refusal")
    );
}
