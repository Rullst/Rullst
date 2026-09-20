use super::*;
use crate::Attachment;
use base64::{Engine, engine::general_purpose::STANDARD};

#[test]
fn native_request_binds_credentials_disables_tracking_and_serializes_inline_assets() {
    let message = Message::new().from("accounts@example.com").to("member@example.com").subject("Notice")
        .html("<img src=\"cid:logo\"><a href=\"https://app.example/reset?token=opaque_123\">Reset</a>")
        .text("Reset").attach(Attachment::inline("logo", "logo.png", vec![1,2,3], "image/png"))
        .unsubscribe_url("https://app.example/unsubscribe");
    let prepared = DeliveryPipeline::prepare(&message).unwrap();
    let request = MailjetDriver::try_new("public-fixture", "private-fixture")
        .unwrap()
        .request(prepared.message())
        .unwrap();
    assert_eq!(request.url().as_str(), "https://api.mailjet.com/v3.1/send");
    assert_eq!(
        request.headers()["authorization"],
        format!(
            "Basic {}",
            STANDARD.encode("public-fixture:private-fixture")
        )
    );
    let body: Value = serde_json::from_slice(request.body().unwrap().as_bytes().unwrap()).unwrap();
    assert_eq!(body["Messages"][0]["TrackOpens"], "disabled");
    assert_eq!(body["Messages"][0]["TrackClicks"], "disabled");
    assert_eq!(
        body["Messages"][0]["InlinedAttachments"][0]["ContentID"],
        "logo"
    );
    assert_eq!(
        body["Messages"][0]["Headers"]["List-Unsubscribe-Post"],
        "List-Unsubscribe=One-Click"
    );
    assert_eq!(body["SandboxMode"], false);
    let receipt = json!({"Messages":[{"Status":"success","To":[{"Email":"member@example.com","MessageID":1,"MessageUUID":"opaque-id"}]}]});
    assert!(validate_receipt(&receipt, false, "member@example.com").is_ok());
    assert!(validate_receipt(&receipt, false, "other@example.com").is_err());
    assert!(
        validate_receipt(
            &json!({"Messages":[{"Status":"error"}]}),
            false,
            "member@example.com"
        )
        .is_err()
    );
    let sandbox = json!({"Messages":[{"Status":"success","To":[{"Email":"member@example.com"}]}]});
    assert!(validate_receipt(&sandbox, true, "member@example.com").is_ok());
    assert!(validate_receipt(&sandbox, false, "member@example.com").is_err());
    assert_eq!(
        payload(prepared.message(), true).unwrap()["SandboxMode"],
        true
    );
}

#[tokio::test]
async fn credential_pair_is_fail_closed_and_empty_pair_is_offline() {
    assert!(MailjetDriver::try_new("real", "").is_err());
    assert!(MailjetDriver::try_new("mock_key", "real").is_err());
    assert!(MailjetDriver::try_new("real", "bad\nsecret").is_err());
    let message = Message::new()
        .from("accounts@example.com")
        .to("member@example.com")
        .subject("Welcome")
        .text("Welcome");
    MailjetDriver::try_new("", "")
        .unwrap()
        .send(&message)
        .await
        .unwrap();
    let scheduled = message.send_at(chrono::Utc::now() + chrono::Duration::minutes(5));
    assert!(
        MailjetDriver::try_new("fixture", "fixture")
            .unwrap()
            .send(&scheduled)
            .await
            .is_err()
    );
}
