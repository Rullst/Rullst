#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use jiff::Timestamp;

#[tokio::test]
async fn global_trap_preserves_prepared_attachments_inline_media_and_schedule() {
    let _guard = crate::facade::MAIL_ENV_LOCK.lock().await;
    MailTrap::clear();
    let scheduled_at = Timestamp::now() + jiff::SignedDuration::from_mins(5);
    let message = Message::new()
        .to("attachments@example.com")
        .from("sender@example.com")
        .subject("Delivery contract")
        .text("plain body")
        .html("<p>plain body</p><img src=\"cid:brand\">")
        .send_at(scheduled_at)
        .attach_bytes("terms.txt", b"terms".to_vec(), "text/plain")
        .attach_cid("brand", "brand.png", vec![1, 2, 3], "image/png");

    MemoryDriver::default().send(&message).await.unwrap();
    assert_eq!(
        MailTrap::last_message()
            .as_ref()
            .map(|mail| mail.to.as_str()),
        Some("attachments@example.com")
    );
    MailTrap::assert_sent_to("ATTACHMENTS@example.com")
        .with_attachment_count(2)
        .with_attachment_named("terms.txt")
        .with_inline_cid("brand")
        .with_scheduled_at(scheduled_at);
    assert!(std::panic::catch_unwind(MailTrap::assert_nothing_sent).is_err());
    MailTrap::clear();
    MailTrap::assert_nothing_sent();
}

#[test]
fn body_assertions_read_either_body_and_absent_files_fail() {
    let message = Message::new()
        .text("text only")
        .html("<p>html only</p>")
        .attach_bytes("terms.txt", b"terms".to_vec(), "text/plain")
        .attach_cid("brand", "brand.png", vec![1, 2, 3], "image/png");
    let assertion = || MailAssertion {
        message: message.clone(),
    };
    assertion()
        .with_body_contains("text only")
        .with_body_contains("html only");
    assert!(std::panic::catch_unwind(|| assertion().with_attachment_named("missing.txt")).is_err());
    assert!(std::panic::catch_unwind(|| assertion().with_inline_cid("missing")).is_err());
}

#[tokio::test]
async fn isolated_driver_reports_a_poisoned_store_without_panicking() {
    let (driver, store) = MemoryDriver::isolated();
    let poison = std::thread::spawn(move || {
        let _guard = store.lock().unwrap();
        panic!("poison isolated fixture");
    });
    assert!(poison.join().is_err());

    let error = driver
        .send(
            &Message::new()
                .to("recipient@example.com")
                .subject("poisoned")
                .text("body"),
        )
        .await
        .unwrap_err();
    assert!(matches!(error, MailError::DriverError(message) if message.contains("poisoned")));
}
