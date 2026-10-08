//! Driver regressions that need only the public API.

use rullst_mail::{
    AwsSesDriver, FailoverDriver, MailDriver, MailError, MemoryDriver, Message, ResendDriver,
    SendPulseDriver,
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
        Box::new(SendPulseDriver::try_new("live-fixture-key").unwrap()),
        Box::new(
            AwsSesDriver::try_new("us-east-1", "live-fixture-token")
                .unwrap()
                .try_with_endpoint("http://127.0.0.1:9/v2/email/outbound-emails")
                .unwrap(),
        ),
    ];
    for driver in drivers {
        let error = driver.send(&message).await.expect_err("no sender");
        // The error names both ways to configure a sender.
        assert!(
            matches!(&error, MailError::ConfigError(text)
                if text.contains("MAIL_FROM") && text.contains("[mail] from")),
            "{error}"
        );
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
async fn resend_rejects_schedules_beyond_its_window_before_network() {
    let message = Message::new()
        .to("user@example.com")
        .from("sender@example.com")
        .subject("Later")
        .text("body")
        .send_in(std::time::Duration::from_secs(31 * 86_400));
    let outcome = ResendDriver::try_new("re_live_fixture")
        .unwrap()
        .send(&message)
        .await;
    assert!(
        matches!(&outcome, Err(MailError::ConfigError(text)) if text.contains("30 days")),
        "{outcome:?}"
    );
    // The offline fixture keeps longer schedules for assertions.
    ResendDriver::try_new("mock_resend")
        .unwrap()
        .send(&message)
        .await
        .unwrap();
}
