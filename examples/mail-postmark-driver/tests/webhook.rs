//! Postmark's `SubscriptionChange` webhook as Rullst suppression events, and
//! the guard that then blocks the recipient before Postmark sees it.

use rullst_mail::{
    InMemorySuppressionStore, MailDriver, MailError, Message, MutableSuppressionStore,
    SuppressionGuard, SuppressionReason,
};
use rullst_mail_postmark_example::{PostmarkDriver, PostmarkWebhook, WebhookError};
use serde_json::{Value, json};

const PASSWORD: &str = "correct-horse-battery-staple";
/// `Basic base64("postmark:correct-horse-battery-staple")`.
const AUTHORIZATION: &str = "Basic cG9zdG1hcms6Y29ycmVjdC1ob3JzZS1iYXR0ZXJ5LXN0YXBsZQ==";

fn webhook() -> PostmarkWebhook {
    PostmarkWebhook::try_new("postmark", PASSWORD).unwrap()
}

/// The documented example payload with `fields` replaced.
fn change(fields: Value) -> Vec<u8> {
    let mut change = json!({
        "RecordType": "SubscriptionChange",
        "MessageID": "883953f4-6105-42a2-a16a-77a8eac79483",
        "ServerID": 23,
        "MessageStream": "outbound",
        "ChangedAt": "2025-11-05T16:33:54.9070259Z",
        "Recipient": "ada@example.com",
        "Origin": "Recipient",
        "SuppressSending": true,
        "SuppressionReason": "HardBounce",
        "Tag": "welcome-email",
        "Metadata": { "PropA": "some value" },
    });
    for (key, value) in fields.as_object().unwrap() {
        change[key] = value.clone();
    }
    serde_json::to_vec(&change).unwrap()
}

fn read(body: &[u8]) -> Result<Option<rullst_mail::SuppressionEvent>, WebhookError> {
    webhook().suppression_event(Some(AUTHORIZATION), body)
}

#[test]
fn credentials_must_be_usable_basic_auth() {
    for (username, password) in [
        ("", PASSWORD),
        ("post:mark", PASSWORD),
        ("post mark", PASSWORD),
        ("post\tmark", PASSWORD),
        ("postmark", "fifteen-chars.."),
        ("postmark", "sixteen chars..."),
    ] {
        assert_eq!(
            PostmarkWebhook::try_new(username, password).err(),
            Some(WebhookError::Configuration),
            "{username:?} {password:?}"
        );
    }
    assert!(PostmarkWebhook::try_new("p", "sixteen-chars...").is_ok());
}

#[test]
fn a_hard_bounce_becomes_a_suppression_event() {
    let event = read(&change(json!({}))).unwrap().unwrap();
    assert_eq!(event.provider(), "postmark");
    assert_eq!(event.recipient(), "ada@example.com");
    assert_eq!(event.reason(), SuppressionReason::HardBounce);
    assert_eq!(event.observed_at(), 1_762_360_434);
    assert_eq!(
        event.event_id(),
        "2025-11-05T16:33:54.907025900Z:b5fc85e55755f9e0d030a10ab4429b6b"
    );
    // A retried delivery is the same event; another recipient of the same
    // change is another one.
    let retried = read(&change(json!({}))).unwrap().unwrap();
    assert_eq!(retried.event_id(), event.event_id());
    let other = read(&change(json!({ "Recipient": "grace@example.com" })))
        .unwrap()
        .unwrap();
    assert_eq!(
        other.event_id(),
        "2025-11-05T16:33:54.907025900Z:b533d4547eaa5a0fa955965a1ca393cc"
    );
    // Any offset is written in UTC.
    let offset = read(&change(
        json!({ "ChangedAt": "2025-11-05T13:33:54-03:00", "MessageID": null }),
    ))
    .unwrap()
    .unwrap();
    assert_eq!(offset.observed_at(), 1_762_360_434);
    assert!(
        offset
            .event_id()
            .starts_with("2025-11-05T16:33:54.000000000Z:")
    );
}

#[test]
fn complaints_and_manual_suppressions_keep_their_reason() {
    for (postmark, reason) in [
        ("SpamComplaint", SuppressionReason::SpamComplaint),
        ("ManualSuppression", SuppressionReason::Manual),
    ] {
        let event = read(&change(json!({ "SuppressionReason": postmark })))
            .unwrap()
            .unwrap();
        assert_eq!(event.reason(), reason, "{postmark}");
    }
}

#[test]
fn reactivations_and_other_records_are_acknowledged_without_an_event() {
    for fields in [
        json!({ "SuppressSending": false, "SuppressionReason": null }),
        json!({ "RecordType": "Bounce" }),
        json!({ "RecordType": "Bounce", "SuppressionReason": null }),
    ] {
        assert_eq!(read(&change(fields.clone())), Ok(None), "{fields}");
    }
}

#[test]
fn requests_without_the_credentials_are_refused_before_the_body_is_read() {
    let lowercase = AUTHORIZATION.replacen("Basic", "basic", 1);
    let wrong = "Basic cG9zdG1hcms6d3Jvbmc=";
    for authorization in [None, Some(""), Some(wrong), Some(lowercase.as_str())] {
        assert_eq!(
            webhook().suppression_event(authorization, b"not json"),
            Err(WebhookError::Unauthorized),
            "{authorization:?}"
        );
    }
}

#[test]
fn malformed_changes_are_invalid_payloads() {
    let invalid = Err(WebhookError::InvalidPayload);
    assert_eq!(read(b"not json"), invalid);
    for fields in [
        json!({ "SuppressionReason": "Unsubscribe" }),
        json!({ "SuppressionReason": null }),
        json!({ "Recipient": null }),
        json!({ "Recipient": "not-an-address" }),
        json!({ "ChangedAt": null }),
        json!({ "ChangedAt": "yesterday" }),
        json!({ "ChangedAt": "1969-12-31T23:59:59Z" }),
        json!({ "ChangedAt": "2999-01-01T00:00:00Z" }),
    ] {
        assert_eq!(read(&change(fields.clone())), invalid, "{fields}");
    }
}

#[test]
fn bodies_are_bounded_at_64_kib() {
    let padded = |length: usize| {
        let mut body = change(json!({}));
        body.resize(length, b' ');
        body
    };
    assert!(read(&padded(64 * 1024)).unwrap().is_some());
    assert_eq!(
        read(&padded(64 * 1024 + 1)),
        Err(WebhookError::InvalidPayload)
    );
}

#[test]
fn errors_explain_themselves_without_the_request() {
    assert_eq!(
        WebhookError::Configuration.to_string(),
        "Postmark webhook credentials are malformed"
    );
    assert_eq!(
        WebhookError::Unauthorized.to_string(),
        "Postmark webhook credentials do not match"
    );
    assert_eq!(
        WebhookError::InvalidPayload.to_string(),
        "Postmark webhook body is not a subscription change"
    );
}

#[tokio::test]
async fn a_recorded_suppression_blocks_the_next_send() {
    let store = InMemorySuppressionStore::new(16, 16).unwrap();
    let event = read(&change(json!({}))).unwrap().unwrap();
    store.record(event).await.unwrap();
    let driver = PostmarkDriver::try_new("mock_postmark").unwrap();
    let outbox = driver.offline_outbox();
    let guarded = SuppressionGuard::new(driver, store);
    let to = |recipient: &str| {
        Message::new()
            .to(recipient)
            .from("hello@acme.example")
            .subject("Hello")
            .text("body")
    };
    assert!(matches!(
        guarded.send(&to("ada@example.com")).await,
        Err(MailError::SuppressedRecipient { .. })
    ));
    guarded.send(&to("grace@example.com")).await.unwrap();
    let captured = outbox.lock().unwrap().clone();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].to, "grace@example.com");
}
