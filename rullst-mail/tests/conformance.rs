//! The mail conformance kit against the built-in drivers (offline mock
//! credentials) and against deliberately broken drivers, each failing its case.
#![cfg(feature = "conformance")]

use async_trait::async_trait;
use rullst_mail::conformance::{
    CaseOutcome, ConformanceReport, DeliveryCapture, OfflineMockCapture, check_mail_driver,
};
use rullst_mail::{
    AwsSesDriver, DeliveryPipeline, MailDriver, MailError, Message, ResendDriver, SendPulseDriver,
};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

#[tokio::test]
async fn resend_passes_offline() {
    let driver = ResendDriver::try_new("mock_resend").unwrap();
    let report = check_mail_driver("resend", &driver, &OfflineMockCapture::new("resend")).await;
    report.assert_passed();
    assert_eq!(report.provider(), "resend");
    assert_eq!(report.cases().len(), 6);
}

#[tokio::test]
async fn aws_ses_passes_offline() {
    let driver = AwsSesDriver::try_new("us-east-1", "mock_ses").unwrap();
    check_mail_driver("aws_ses", &driver, &OfflineMockCapture::new("aws_ses"))
        .await
        .assert_passed();
}

#[tokio::test]
async fn sendpulse_passes_offline() {
    let driver = SendPulseDriver::try_new("mock_sendpulse").unwrap();
    check_mail_driver("sendpulse", &driver, &OfflineMockCapture::new("sendpulse"))
        .await
        .assert_passed();
}

#[cfg(feature = "mail-smtp")]
#[tokio::test]
async fn smtp_passes_offline() {
    let driver = rullst_mail::SmtpDriver::try_new("mock_smtp", 587, None, None).unwrap();
    check_mail_driver("smtp", &driver, &OfflineMockCapture::new("smtp"))
        .await
        .assert_passed();
}

#[tokio::test]
async fn a_capture_of_another_provider_sees_nothing() {
    let driver = ResendDriver::try_new("mock_resend").unwrap();
    let report = check_mail_driver("resend", &driver, &OfflineMockCapture::new("smtp")).await;
    assert_eq!(outcome(&report, "delivery.accepted"), CaseOutcome::Failed);
    assert_eq!(
        outcome(&report, "headers.crlf_refused"),
        CaseOutcome::Passed
    );
}

/// Ways a driver can break the contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Defect {
    SkipsPipeline,
    WrongErrorType,
    DropsMessages,
    ChangesRecipient,
    DropsAttachments,
    NonDeterministic,
    IgnoresDeliveryId,
    IgnoresTenant,
    Panics,
}

/// A transport that keeps what it would send in memory, with an optional
/// defect. It is its own capture.
struct Probe {
    defect: Option<Defect>,
    outbox: Mutex<Vec<Message>>,
    sends: AtomicU32,
}

impl Probe {
    fn new(defect: Option<Defect>) -> Self {
        Self {
            defect,
            outbox: Mutex::new(Vec::new()),
            sends: AtomicU32::new(0),
        }
    }
}

impl DeliveryCapture for Probe {
    fn captured(&self) -> Result<Vec<Message>, MailError> {
        Ok(self.outbox.lock().unwrap().clone())
    }
}

#[async_trait]
impl MailDriver for Probe {
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        let mut message = match self.defect {
            Some(Defect::Panics) => panic!("probe send panic"),
            Some(Defect::SkipsPipeline) => message.clone(),
            Some(Defect::WrongErrorType) => DeliveryPipeline::prepare(message)
                .map_err(|_| MailError::TransportError {
                    provider: "probe",
                    message: "refused".into(),
                })?
                .into_message(),
            _ => DeliveryPipeline::prepare(message)?.into_message(),
        };
        match self.defect {
            Some(Defect::DropsMessages) => return Ok(()),
            Some(Defect::ChangesRecipient) => message.to = "someone.else@example.com".into(),
            Some(Defect::DropsAttachments) => message.attachments.clear(),
            Some(Defect::NonDeterministic) => {
                let copy = self.sends.fetch_add(1, Ordering::Relaxed);
                message.body_text = Some(format!("copy {copy}"));
            }
            _ => {}
        }
        self.outbox.lock().unwrap().push(message);
        Ok(())
    }

    async fn send_with_delivery_id(&self, message: &Message, id: &str) -> Result<(), MailError> {
        let valid = (16..=128).contains(&id.len())
            && id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'));
        if !valid && self.defect != Some(Defect::IgnoresDeliveryId) {
            return Err(MailError::ValidationError("delivery ID".into()));
        }
        self.send(message).await
    }

    async fn send_for_tenant(&self, tenant: &str, message: &Message) -> Result<(), MailError> {
        if self.defect == Some(Defect::IgnoresTenant) {
            return self.send(message).await;
        }
        let prepared = DeliveryPipeline::prepare_for_tenant(tenant, message)?;
        self.send(prepared.message()).await
    }
}

fn outcome(report: &ConformanceReport, id: &str) -> CaseOutcome {
    report.case(id).map(|case| case.outcome()).unwrap()
}

#[tokio::test]
async fn an_application_transport_that_runs_the_pipeline_passes() {
    let probe = Probe::new(None);
    let report = check_mail_driver("probe", &probe, &probe).await;
    report.assert_passed();
    assert!(report.cases().iter().all(|case| case.detail().is_empty()));
    // accepted 3, attachments 1, suppression 3, deterministic 2.
    assert_eq!(probe.captured().unwrap().len(), 9);
}

#[tokio::test]
async fn each_defect_fails_its_case() {
    use Defect::*;
    for (defect, id) in [
        (SkipsPipeline, "headers.crlf_refused"),
        (SkipsPipeline, "attachments.limits_enforced"),
        (SkipsPipeline, "errors.typed_no_panic"),
        (WrongErrorType, "headers.crlf_refused"),
        (DropsMessages, "delivery.accepted"),
        (DropsMessages, "suppression.respected"),
        (ChangesRecipient, "delivery.accepted"),
        (DropsAttachments, "attachments.limits_enforced"),
        (NonDeterministic, "offline.deterministic"),
        (IgnoresDeliveryId, "errors.typed_no_panic"),
        (IgnoresTenant, "errors.typed_no_panic"),
        (Panics, "delivery.accepted"),
    ] {
        let probe = Probe::new(Some(defect));
        let report = check_mail_driver("probe", &probe, &probe).await;
        assert_eq!(
            outcome(&report, id),
            CaseOutcome::Failed,
            "{defect:?}\n{report}"
        );
        assert!(!report.passed(), "{defect:?}");
    }
}

#[tokio::test]
async fn a_panic_fails_its_case_and_the_report_lists_every_case() {
    let probe = Probe::new(Some(Defect::Panics));
    let report = check_mail_driver("probe", &probe, &probe).await;
    assert_eq!(
        report.case("delivery.accepted").unwrap().detail(),
        "panicked: probe send panic"
    );
    assert_eq!(report.failures().count(), 6);

    let probe = Probe::new(Some(Defect::IgnoresTenant));
    let report = check_mail_driver("probe", &probe, &probe).await;
    let text = report.to_string();
    assert!(
        text.starts_with(
            "conformance of probe: 5 passed, 1 failed, 0 skipped\n  ok delivery.accepted\n"
        ),
        "{text}"
    );
    assert!(
        text.ends_with("  FAILED errors.typed_no_panic: [tenant-format] is a ValidationError, not Ok(()); [tenant-format] is captured 0 time(s), not 1\n"),
        "{text}"
    );
    assert_eq!(
        report.failures().map(|case| case.id()).collect::<Vec<_>>(),
        ["errors.typed_no_panic"]
    );
    assert!(report.case("no.such.case").is_none());
}

#[tokio::test]
#[should_panic(expected = "FAILED offline.deterministic")]
async fn assert_passed_fails_the_test_with_the_report() {
    let probe = Probe::new(Some(Defect::NonDeterministic));
    check_mail_driver("probe", &probe, &probe)
        .await
        .assert_passed();
}
