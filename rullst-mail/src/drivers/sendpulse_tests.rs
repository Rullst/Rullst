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
    let request = SendPulseDriver::try_new("fixture_sendpulse")
        .unwrap()
        .request(prepared.message())
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
    let scheduled = message.send_at(chrono::Utc::now() + chrono::Duration::minutes(5));
    assert!(
        SendPulseDriver::try_new("fixture_sendpulse")
            .unwrap()
            .send(&scheduled)
            .await
            .is_err()
    );
}
