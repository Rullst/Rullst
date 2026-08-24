#![allow(clippy::unwrap_used, clippy::expect_used)]

use rullst_mail::Message;
use rullst_mail::tracking::{PIXEL_1X1_GIF, TrackingEngine};
use rullst_mail::validator::{
    extract_domain, is_disposable_domain, validate_email_deliverability, validate_email_syntax,
};
use std::time::Duration;

#[test]
fn test_deliverability_and_disposable_domains() {
    // Valid emails
    assert!(validate_email_deliverability("alice@acme-corp.com").is_ok());
    assert!(validate_email_syntax("user.name+tag@example.co.uk").is_ok());

    // Domain extraction
    assert_eq!(extract_domain("john@company.org"), Some("company.org"));
    assert_eq!(extract_domain("invalid-no-at"), None);

    // Disposable domains filter
    assert!(is_disposable_domain("10minutemail.com"));
    assert!(is_disposable_domain("mailinator.com") || is_disposable_domain("temp-mail.org"));
    assert!(!is_disposable_domain("gmail.com"));
    assert!(!is_disposable_domain("protonmail.com"));

    // Validation catches disposable
    assert!(validate_email_deliverability("test@10minutemail.com").is_err());
}

#[test]
fn test_tracking_engine_open_and_click_tokens() {
    let secret = b"rullst-mail-integration-key-32-bytes-2026";

    // 1. Open tracking
    let timestamp = 1_724_450_000;
    let token = TrackingEngine::try_generate_open_token(
        secret,
        "user@example.com",
        "newsletter_august",
        timestamp,
    )
    .unwrap();
    let verified =
        TrackingEngine::verify_open_token_at(secret, &token, timestamp, Duration::from_secs(60))
            .unwrap();
    assert_eq!(verified.email, "user@example.com");
    assert_eq!(verified.campaign_id, "newsletter_august");

    // Invalid secret verification fails
    assert!(
        TrackingEngine::verify_open_token_at(
            b"wrong_secret",
            &token,
            timestamp,
            Duration::from_secs(60)
        )
        .is_err()
    );

    // 2. Click tracking
    let click_token = TrackingEngine::try_generate_click_token(
        secret,
        "user@example.com",
        "https://example.com/checkout",
        timestamp,
    )
    .unwrap();
    let verified_click = TrackingEngine::verify_click_token_at(
        secret,
        &click_token,
        timestamp,
        Duration::from_secs(60),
    )
    .unwrap();
    assert_eq!(verified_click.target_url, "https://example.com/checkout");

    // 3. Pixel injection & verification
    let html = "<html><body><p>Hello</p></body></html>";
    let injected =
        TrackingEngine::try_inject_open_pixel(html, "https://track.example.com/pixel.gif").unwrap();
    assert!(injected.contains("https://track.example.com/pixel.gif"));

    assert_eq!(PIXEL_1X1_GIF.len(), 43);
    assert_eq!(&PIXEL_1X1_GIF[0..6], b"GIF89a");
}

#[test]
fn test_message_builder_and_body() {
    let msg = Message::new()
        .to("recipient@example.com")
        .from("noreply@rullst.dev")
        .subject("Welcome to Rullst")
        .text("Hello World")
        .html("<p>Hello World</p>");

    assert_eq!(msg.to, "recipient@example.com");
    assert_eq!(msg.subject, "Welcome to Rullst");
    assert_eq!(msg.body_text.unwrap(), "Hello World");
    assert_eq!(msg.body_html.unwrap(), "<p>Hello World</p>");
}
