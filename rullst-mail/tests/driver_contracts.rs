//! Driver regressions that need only the public API.

use rullst_mail::{
    AwsSesDriver, FailoverDriver, MailDriver, MailError, MemoryDriver, Message, OfflineMailMock,
    ResendDriver, SendPulseDriver,
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
    // The offline mock refuses it too, so a test sees what production would.
    let offline = ResendDriver::try_new("mock_resend")
        .unwrap()
        .send(&message)
        .await;
    assert!(
        matches!(&offline, Err(MailError::ConfigError(text)) if text.contains("30 days")),
        "{offline:?}"
    );
}

#[tokio::test]
async fn the_offline_mock_retains_64_mib_and_clear_removes_every_delivery() {
    // The only test in this binary that reads or clears the offline store.
    let driver = ResendDriver::try_new("mock_resend").unwrap();
    let body = "x".repeat(600_000);
    let subjects = ["offline bound first", "offline bound second"];
    for subject in subjects {
        let message = Message::new()
            .to("user@example.com")
            .from("team@example.com")
            .subject(subject)
            .text(body.clone());
        driver.send(&message).await.unwrap();
    }
    let retained = |subject: &str| {
        OfflineMailMock::deliveries()
            .unwrap()
            .iter()
            .any(|delivery| delivery.message.subject == subject)
    };
    // 1.2 MB of bodies stays well inside the 64 MiB retention bound.
    assert!(subjects.iter().all(|subject| retained(subject)));
    OfflineMailMock::clear().unwrap();
    assert!(!subjects.iter().any(|subject| retained(subject)));
}

#[tokio::test]
async fn resend_accepts_schedules_inside_its_window() {
    // Without a sender the request stops before network, after the window check.
    let message = Message::new()
        .to("user@example.com")
        .subject("Soon")
        .text("body")
        .send_in(std::time::Duration::from_secs(86_400));
    let outcome = ResendDriver::try_new("re_live_fixture")
        .unwrap()
        .send(&message)
        .await;
    assert!(
        matches!(&outcome, Err(MailError::ConfigError(text)) if text.contains("MAIL_FROM")),
        "{outcome:?}"
    );
}
