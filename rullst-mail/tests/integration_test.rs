// tests/integration_test.rs — Comprehensive unit and integration tests for Rullst Mail.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use rullst_mail::drivers::failover::FailoverDriver;
use rullst_mail::drivers::memory::MemoryDriver;
use rullst_mail::drivers::{LogDriver, MailDriver};
use rullst_mail::message::Message;
use rullst_mail::security::{extract_urls, is_dangerous_scheme, is_homograph_domain};
use rullst_mail::tracking::{TrackingEngine, TrackingError};

#[tokio::test]
async fn test_memory_driver_and_failover() {
    let (memory_driver, store) = MemoryDriver::isolated();

    let msg = Message::new()
        .to("customer@example.com")
        .from("notifications@rullst.com")
        .subject("Your weekly report")
        .html("<p>Here is your weekly report</p>");

    // Send via memory driver
    let res = memory_driver.send(&msg).await;
    assert!(res.is_ok());

    let sent = store.lock().unwrap().clone();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].to, "customer@example.com");
    assert_eq!(sent[0].subject, "Your weekly report");

    // Failover driver with memory backup
    let failover = FailoverDriver::new(LogDriver)
        .with_fallback(MemoryDriver::new());
    assert!(failover.send(&msg).await.is_ok());
}

#[test]
fn test_email_tracking_tokens_and_pixel_injection() {
    let secret = b"my_super_secret_hmac_tracking_key";
    let email = "user@test.org";
    let campaign = "launch_2026";
    let timestamp = 1700000000;

    // 1. Open tracking
    let open_token = TrackingEngine::generate_open_token(secret, email, campaign, timestamp);
    assert!(!open_token.is_empty());

    let verified_open = TrackingEngine::verify_open_token(secret, &open_token).unwrap();
    assert_eq!(verified_open.email, email);
    assert_eq!(verified_open.campaign_id, campaign);
    assert_eq!(verified_open.timestamp, timestamp);

    // Invalid open token verification
    assert_eq!(
        TrackingEngine::verify_open_token(b"wrong_secret", &open_token).unwrap_err(),
        TrackingError::InvalidSignature
    );

    // 2. Click tracking
    let target = "https://rullst.dev/docs";
    let click_token = TrackingEngine::generate_click_token(secret, email, target, timestamp);
    let verified_click = TrackingEngine::verify_click_token(secret, &click_token).unwrap();
    assert_eq!(verified_click.email, email);
    assert_eq!(verified_click.target_url, target);

    // 3. Pixel injection
    let html = "<html><body><h1>Hello</h1></body></html>";
    let tracked_html = TrackingEngine::inject_open_pixel(html, "https://rullst.dev/t/pixel.gif");
    assert!(tracked_html.contains("<img src=\"https://rullst.dev/t/pixel.gif\""));
    assert!(tracked_html.contains("</body>"));
}

#[test]
fn test_security_scanner_and_homograph_detection() {
    // 1. Homograph domain detection (Latin vs Cyrillic)
    assert!(is_homograph_domain("p\u{0430}ypal.com")); // Cyrillic 'а' mixed with Latin
    assert!(!is_homograph_domain("paypal.com")); // Pure Latin
    assert!(!is_homograph_domain("rullst.dev")); // Pure Latin

    // 2. Dangerous URI schemes
    assert!(is_dangerous_scheme("javascript:alert(1)"));
    assert!(is_dangerous_scheme("data:text/html;base64,PHNjcmlwdD5hbGVydCgxKTwvc2NyaXB0Pg=="));
    assert!(is_dangerous_scheme("file:///etc/passwd"));
    assert!(!is_dangerous_scheme("https://rullst.dev"));
    assert!(!is_dangerous_scheme("http://localhost:3000"));

    // 3. Extract URLs
    let html_content = r#"
        <a href="https://example.com/login">Login here</a>
        <a href="javascript:void(0)">Click</a>
    "#;
    let urls = extract_urls(html_content);
    assert!(urls.contains(&"https://example.com/login".to_string()));
    assert!(urls.contains(&"javascript:void(0)".to_string()));
}
