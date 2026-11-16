//! The `rullst-mail` conformance kit against the offline mode, as for the
//! built-in Resend, SES, SendPulse and SMTP drivers.

use rullst_mail::conformance::{DeliveryCapture, check_mail_driver};
use rullst_mail::{MailError, Message};
use rullst_mail_postmark_example::{OfflineOutbox, PROVIDER, PostmarkDriver};

/// The driver's offline outbox, read by the kit.
struct Outbox(OfflineOutbox);

impl DeliveryCapture for Outbox {
    fn captured(&self) -> Result<Vec<Message>, MailError> {
        self.0
            .lock()
            .map(|messages| messages.clone())
            .map_err(|_| MailError::DriverError("outbox lock poisoned".into()))
    }
}

#[tokio::test]
async fn postmark_passes_the_conformance_kit_offline() {
    for token in ["mock_postmark", ""] {
        let driver = PostmarkDriver::try_new(token).unwrap();
        let report = check_mail_driver(PROVIDER, &driver, &Outbox(driver.offline_outbox())).await;
        report.assert_passed();
        assert_eq!(report.provider(), "postmark");
        assert_eq!(report.cases().len(), 6);
    }
}
