//! Driver regressions that need only the public API.

use rullst_mail::{
    FailoverDriver, MailDriver, MailError, MemoryDriver, Message, PostmarkDriver, ResendDriver,
    SendGridDriver,
};

/// Always returns the configured error.
struct Failing(MailError);

#[async_trait::async_trait]
impl MailDriver for Failing {
    async fn send(&self, _message: &Message) -> Result<(), MailError> {
        Err(self.0.clone())
    }
}

#[tokio::test]
async fn real_transports_require_an_explicit_sender_before_network() {
    let message = Message::new()
        .to("user@example.com")
        .subject("No sender")
        .text("body");
    let drivers: Vec<Box<dyn MailDriver>> = vec![
        Box::new(ResendDriver::try_new("re_live_fixture").unwrap()),
        Box::new(SendGridDriver::try_new("SG.live_fixture").unwrap()),
        Box::new(PostmarkDriver::try_new("live-fixture-token").unwrap()),
    ];
    for driver in drivers {
        assert!(matches!(
            driver.send(&message).await,
            Err(MailError::ConfigError(_))
        ));
    }
}

#[tokio::test]
async fn guard_outages_do_not_fail_over_to_an_unguarded_provider() {
    let message = Message::new()
        .to("user@example.com")
        .subject("Guard outage");
    for error in [
        MailError::SuppressionUnavailable,
        MailError::AttachmentInspectionUnavailable,
    ] {
        let (fallback, fallback_store) = MemoryDriver::isolated();
        let failover = FailoverDriver::new(Failing(error.clone())).with_fallback(fallback);
        let outcome = failover.send(&message).await;
        assert_eq!(outcome, Err(error));
        assert!(fallback_store.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn sendgrid_rejects_schedules_beyond_its_window_before_network() {
    let message = Message::new()
        .to("user@example.com")
        .from("sender@example.com")
        .subject("Later")
        .text("body")
        .send_in(std::time::Duration::from_secs(5 * 86_400));
    let outcome = SendGridDriver::try_new("SG.live_fixture")
        .unwrap()
        .send(&message)
        .await;
    assert!(matches!(outcome, Err(MailError::ConfigError(_))));
    // The offline fixture keeps longer schedules for assertions.
    SendGridDriver::try_new("mock_sendgrid")
        .unwrap()
        .send(&message)
        .await
        .unwrap();
}
