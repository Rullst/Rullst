#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::*;

#[test]
fn test_message_to() {
    let msg = Message::new().to("user@example.com");
    assert_eq!(msg.to, "user@example.com");
}

#[tokio::test]
async fn test_mail_custom() {
    let msg = Message::new()
        .to("a")
        .from("b")
        .subject("c")
        .text("d")
        .html("e");
    assert_eq!(msg.to, "a");
    assert_eq!(msg.from.unwrap(), "b");
}

#[tokio::test]
async fn test_attachments_and_inline_cid() {
    MailTrap::clear();
    let trap = MailTrap::driver();

    let pdf_data = b"%PDF-1.4 test invoice content";
    let logo_data = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR";

    let msg = Message::new()
        .to("billing@client.com")
        .subject("Invoice Attached")
        .html("<p>Please find attached.</p><img src=\"cid:brand_logo\" />")
        .attach_bytes("invoice.pdf", pdf_data.to_vec(), "application/pdf")
        .attach_cid("brand_logo", "logo.png", logo_data.to_vec(), "image/png");

    assert_eq!(msg.attachments.len(), 2);
    assert_eq!(msg.attachments[0].filename, "invoice.pdf");
    assert!(!msg.attachments[0].is_inline());
    assert_eq!(msg.attachments[1].cid.as_deref(), Some("brand_logo"));
    assert!(msg.attachments[1].is_inline());

    let res = trap.send(&msg).await;
    assert!(res.is_ok());

    MailTrap::assert_sent_to("billing@client.com")
        .with_attachment_count(2)
        .with_attachment_named("invoice.pdf")
        .with_inline_cid("brand_logo");
}

#[tokio::test]
async fn test_scheduled_delivery_send_at() {
    MailTrap::clear();
    let trap = MailTrap::driver();

    let target_time = chrono::Utc::now() + chrono::Duration::hours(24);
    let msg = Message::new()
        .to("future@example.com")
        .subject("Scheduled Update")
        .send_at(target_time);

    assert_eq!(msg.send_at, Some(target_time));

    let res = trap.send(&msg).await;
    assert!(res.is_ok());

    MailTrap::assert_sent_to("future@example.com").with_scheduled_at(target_time);

    let in_msg = Message::new().send_in(std::time::Duration::from_secs(3600));
    assert!(in_msg.send_at.is_some());
}

#[test]
fn test_security_homograph_and_dangerous_schemes() {
    // Pure domain checks
    assert!(!is_homograph_domain("paypal.com"));
    assert!(!is_homograph_domain("google.com"));
    assert!(!is_homograph_domain("rullst.dev"));

    // Homograph attack: Cyrillic 'а' (\u{0430}) inside Latin "paypal.com"
    let spoofed_paypal = "p\u{0430}ypal.com";
    assert!(is_homograph_domain(spoofed_paypal));

    // Dangerous URI schemes
    assert!(is_dangerous_scheme("javascript:alert(1)"));
    assert!(is_dangerous_scheme("data:text/html;base64,PHNjcmlwdD4="));
    assert!(is_dangerous_scheme("vbscript:execute()"));
    assert!(!is_dangerous_scheme("https://rullst.dev/verify"));
    assert!(!is_dangerous_scheme("mailto:support@rullst.dev"));

    // Validate security on Message
    let safe_msg = Message::new()
        .to("safe@example.com")
        .html("<p>Welcome! Click <a href=\"https://rullst.dev/login\">here</a></p>");
    assert!(safe_msg.validate_security().is_ok());

    let bad_scheme_msg = Message::new()
        .to("victim@example.com")
        .html("<p><a href=\"javascript:evil()\">Claim Prize</a></p>");
    assert!(bad_scheme_msg.validate_security().is_err());

    let homograph_msg = Message::new().to("victim@example.com").html(format!(
        "<a href=\"https://{}/login\">Verify</a>",
        spoofed_paypal
    ));
    assert!(homograph_msg.validate_security().is_err());
}
