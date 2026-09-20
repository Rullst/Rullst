use super::*;
use crate::Attachment;

#[test]
fn production_and_sandbox_have_distinct_fixed_endpoints_and_verified_receipts() {
    let message = Message::new().from("accounts@example.com").to("member@example.com").subject("Notice")
        .html("<img src=\"cid:logo\"><a href=\"https://app.example/reset?token=opaque_123\">Reset</a>")
        .text("Reset").attach(Attachment::inline("logo", "logo.png", vec![1,2,3], "image/png"));
    let prepared = DeliveryPipeline::prepare(&message).unwrap();
    for (driver, url) in [
        (
            MailtrapDriver::try_new("fixture_token").unwrap(),
            "https://send.api.mailtrap.io/api/send",
        ),
        (
            MailtrapDriver::sandbox("fixture_token", 42).unwrap(),
            "https://sandbox.api.mailtrap.io/api/send/42",
        ),
    ] {
        let request = driver.request(prepared.message()).unwrap();
        assert_eq!(request.url().as_str(), url);
        assert_eq!(request.headers()["authorization"], "Bearer fixture_token");
        let body: Value =
            serde_json::from_slice(request.body().unwrap().as_bytes().unwrap()).unwrap();
        assert_eq!(body["attachments"][0]["content_id"], "logo");
        assert_eq!(body["attachments"][0]["content"], "AQID");
        assert!(body["html"].as_str().unwrap().contains("token=opaque_123"));
    }
    assert!(validate_receipt(&json!({"success":true,"message_ids":["opaque-id"]})).is_ok());
    for bad in [
        json!({"success":false,"message_ids":["opaque-id"]}),
        json!({"success":true}),
        json!({"success":true,"message_ids":["one","two"]}),
    ] {
        assert!(validate_receipt(&bad).is_err());
    }
    assert!(MailtrapDriver::sandbox("", 0).is_err());
}

#[tokio::test]
async fn empty_tokens_are_offline_but_invalid_and_future_input_is_rejected() {
    let message = Message::new()
        .from("accounts@example.com")
        .to("member@example.com")
        .subject("Welcome")
        .text("Welcome");
    for driver in [
        MailtrapDriver::try_new("").unwrap(),
        MailtrapDriver::sandbox("mock_token", 42).unwrap(),
    ] {
        driver.send(&message).await.unwrap();
        assert!(
            driver
                .send(&message.clone().to("invalid\naddress"))
                .await
                .is_err()
        );
    }
    assert!(MailtrapDriver::try_new("bad\ntoken").is_err());
    let scheduled = message.send_at(chrono::Utc::now() + chrono::Duration::minutes(5));
    assert!(
        MailtrapDriver::try_new("fixture")
            .unwrap()
            .send(&scheduled)
            .await
            .is_err()
    );
}
