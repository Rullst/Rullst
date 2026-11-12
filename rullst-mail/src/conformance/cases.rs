//! The cases of the mail conformance kit, one method each.

use super::DeliveryCapture;
use super::report::Check;
use crate::{
    InMemorySuppressionStore, MAX_ATTACHMENT_BYTES, MAX_ATTACHMENT_COUNT, MailDriver, MailError,
    Message, MutableSuppressionStore, SuppressionEvent, SuppressionGuard, SuppressionReason,
};
use async_trait::async_trait;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const RECIPIENT: &str = "conformance.recipient@example.com";
const SUPPRESSED: &str = "conformance.suppressed@example.com";
const SENDER: &str = "conformance.sender@example.com";
const DELIVERY_ID: &str = "conformance_delivery_0001";
const TENANT: &str = "conformance-tenant";
const INJECTION: &str = "\r\nBcc: attacker@example.com";
/// Under the 20 MiB per-attachment limit; two exceed the 25 MiB total.
const HALF_TOTAL: usize = 13_631_488;

static RUNS: AtomicU64 = AtomicU64::new(0);

/// One kit run: the driver, its capture and the run's subject token.
pub(super) struct Run<'a, D: ?Sized, C: ?Sized> {
    driver: &'a D,
    capture: &'a C,
    id: String,
}

impl<'a, D, C> Run<'a, D, C>
where
    D: MailDriver + ?Sized,
    C: DeliveryCapture + ?Sized,
{
    pub(super) fn new(driver: &'a D, capture: &'a C) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos());
        let id = format!("{nanos}-{}", RUNS.fetch_add(1, Ordering::Relaxed));
        Self {
            driver,
            capture,
            id,
        }
    }

    fn subject(&self, label: &str) -> String {
        format!("Conformance {} [{label}]", self.id)
    }

    /// A valid message whose subject names this run and `label`.
    fn message(&self, label: &str) -> Message {
        Message::new()
            .to(RECIPIENT)
            .from(SENDER)
            .subject(self.subject(label))
            .text("A conformance check message.")
    }

    fn captured(&self, label: &str) -> Result<Vec<Message>, MailError> {
        let subject = self.subject(label);
        Ok(self
            .capture
            .captured()?
            .into_iter()
            .filter(|message| message.subject.starts_with(&subject))
            .collect())
    }

    /// Expects `count` captured messages for `label`; returns them.
    fn expect_captured(&self, check: &mut Check, label: &str, count: usize) -> Vec<Message> {
        match self.captured(label) {
            Ok(messages) => {
                check.expect(
                    messages.len() == count,
                    format!(
                        "[{label}] is captured {count} time(s), not {}",
                        messages.len()
                    ),
                );
                messages
            }
            Err(error) => {
                check.expect(false, format!("the capture lists its messages: {error}"));
                Vec::new()
            }
        }
    }

    /// Expects a `ValidationError` and nothing captured for `label`.
    fn expect_refused(&self, check: &mut Check, label: &str, result: Result<(), MailError>) {
        check.expect(
            matches!(result, Err(MailError::ValidationError(_))),
            format!("[{label}] is a ValidationError, not {result:?}"),
        );
        self.expect_captured(check, label, 0);
    }

    pub(super) async fn accepted(&self) -> Check {
        let mut check = Check::default();
        let driver = self.driver;
        for (label, result) in [
            ("send", driver.send(&self.message("send")).await),
            (
                "delivery-id",
                driver
                    .send_with_delivery_id(&self.message("delivery-id"), DELIVERY_ID)
                    .await,
            ),
            (
                "tenant",
                driver
                    .send_for_tenant(TENANT, &self.message("tenant"))
                    .await,
            ),
        ] {
            check.expect(
                result.is_ok(),
                format!("[{label}] delivers a valid message, not {result:?}"),
            );
            for message in self.expect_captured(&mut check, label, 1) {
                check.expect(
                    message.to == RECIPIENT,
                    format!("[{label}] keeps its recipient"),
                );
            }
        }
        check
    }

    pub(super) async fn crlf_refused(&self) -> Check {
        let mut check = Check::default();
        let lf_subject = format!("{}\nBcc: attacker@example.com", self.subject("subject-lf"));
        for (label, message) in [
            (
                "subject",
                self.message("subject")
                    .subject(format!("{}{INJECTION}", self.subject("subject"))),
            ),
            ("subject-lf", self.message("subject-lf").subject(lf_subject)),
            (
                "to",
                self.message("to").to(format!("{RECIPIENT}{INJECTION}")),
            ),
            (
                "from",
                self.message("from").from(format!("{SENDER}{INJECTION}")),
            ),
            (
                "reply-to",
                self.message("reply-to")
                    .reply_to(format!("{SENDER}{INJECTION}")),
            ),
        ] {
            let result = self.driver.send(&message).await;
            self.expect_refused(&mut check, label, result);
        }
        check
    }

    pub(super) async fn attachment_limits(&self) -> Check {
        let mut check = Check::default();
        let tiny = || vec![b'x'];
        let half = || vec![0_u8; HALF_TOTAL];
        let too_many = (0..=MAX_ATTACHMENT_COUNT).fold(self.message("count"), |message, index| {
            message.attach_bytes(format!("file-{index}.txt"), tiny(), "text/plain")
        });
        for (label, message) in [
            ("count", too_many),
            (
                "size",
                self.message("size").attach_bytes(
                    "large.bin",
                    vec![0_u8; MAX_ATTACHMENT_BYTES + 1],
                    "application/octet-stream",
                ),
            ),
            (
                "total",
                self.message("total")
                    .attach_bytes("first.bin", half(), "application/octet-stream")
                    .attach_bytes("second.bin", half(), "application/octet-stream"),
            ),
            (
                "filename",
                self.message("filename")
                    .attach_bytes("a\r\nb.txt", tiny(), "text/plain"),
            ),
            (
                "path",
                self.message("path")
                    .attach_bytes("../escape.txt", tiny(), "text/plain"),
            ),
        ] {
            let result = self.driver.send(&message).await;
            self.expect_refused(&mut check, label, result);
        }
        let within =
            self.message("within")
                .attach_bytes("notes.txt", b"notes".to_vec(), "text/plain");
        let result = self.driver.send(&within).await;
        check.expect(
            result.is_ok(),
            format!("[within] delivers one small attachment, not {result:?}"),
        );
        for message in self.expect_captured(&mut check, "within", 1) {
            check.expect(
                message.attachments == within.attachments,
                "[within] keeps its attachment",
            );
        }
        check
    }

    pub(super) async fn suppression(&self) -> Check {
        let mut check = Check::default();
        let guard = match suppressing(SUPPRESSED).await {
            Ok(store) => SuppressionGuard::new(Borrowed(self.driver), store),
            Err(error) => {
                check.expect(
                    false,
                    format!("an in-memory suppression store records the recipient: {error}"),
                );
                return check;
            }
        };
        for (label, recipient, path) in [
            ("suppressed-send", SUPPRESSED, Path::Send),
            ("suppressed-delivery-id", SUPPRESSED, Path::DeliveryId),
            ("suppressed-tenant", SUPPRESSED, Path::Tenant),
            ("allowed-send", RECIPIENT, Path::Send),
            ("allowed-delivery-id", RECIPIENT, Path::DeliveryId),
            ("allowed-tenant", RECIPIENT, Path::Tenant),
        ] {
            let message = self.message(label).to(recipient);
            let result = match path {
                Path::Send => guard.send(&message).await,
                Path::DeliveryId => guard.send_with_delivery_id(&message, DELIVERY_ID).await,
                Path::Tenant => guard.send_for_tenant(TENANT, &message).await,
            };
            if recipient == SUPPRESSED {
                check.expect(
                    matches!(result, Err(MailError::SuppressedRecipient { .. })),
                    format!("[{label}] is SuppressedRecipient, not {result:?}"),
                );
                self.expect_captured(&mut check, label, 0);
            } else {
                check.expect(
                    result.is_ok(),
                    format!("[{label}] reaches the driver through the guard, not {result:?}"),
                );
                self.expect_captured(&mut check, label, 1);
            }
        }
        check
    }

    pub(super) async fn deterministic(&self) -> Check {
        let mut check = Check::default();
        let message = self.message("twice");
        for attempt in ["first", "second"] {
            let result = self.driver.send(&message).await;
            check.expect(
                result.is_ok(),
                format!("the {attempt} copy is delivered, not {result:?}"),
            );
        }
        let copies = self.expect_captured(&mut check, "twice", 2);
        check.expect(
            copies.windows(2).all(|pair| pair[0] == pair[1]),
            "the same message is captured identically twice",
        );
        check
    }

    pub(super) async fn typed_errors(&self) -> Check {
        let mut check = Check::default();
        let driver = self.driver;
        let long = format!("{} {}", self.subject("long-subject"), "x".repeat(4096));
        for (label, result) in [
            (
                "empty-recipient",
                driver.send(&self.message("empty-recipient").to("")).await,
            ),
            (
                "invalid-recipient",
                driver
                    .send(&self.message("invalid-recipient").to("not-an-address"))
                    .await,
            ),
            (
                "long-subject",
                driver
                    .send(&self.message("long-subject").subject(long))
                    .await,
            ),
            (
                "delivery-id-format",
                driver
                    .send_with_delivery_id(&self.message("delivery-id-format"), "short")
                    .await,
            ),
            (
                "tenant-format",
                driver
                    .send_for_tenant("../escape", &self.message("tenant-format"))
                    .await,
            ),
        ] {
            self.expect_refused(&mut check, label, result);
        }
        check
    }
}

/// The guard method a suppression check sends through.
#[derive(Clone, Copy)]
enum Path {
    Send,
    DeliveryId,
    Tenant,
}

/// A process-local store in which `recipient` is suppressed.
async fn suppressing(recipient: &str) -> Result<InMemorySuppressionStore, String> {
    let store = InMemorySuppressionStore::new(16, 16).map_err(|error| error.to_string())?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(1, |elapsed| elapsed.as_secs());
    let event = SuppressionEvent::try_new(
        "conformance",
        "conformance-event",
        recipient,
        SuppressionReason::HardBounce,
        now,
    )
    .map_err(|error| error.to_string())?;
    store
        .record(event)
        .await
        .map_err(|error| error.to_string())?;
    Ok(store)
}

/// Lends the driver under test to a [`SuppressionGuard`].
struct Borrowed<'a, D: ?Sized>(&'a D);

#[async_trait]
impl<'a, D: MailDriver + ?Sized> MailDriver for Borrowed<'a, D> {
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        self.0.send(message).await
    }

    async fn send_with_delivery_id(
        &self,
        message: &Message,
        delivery_id: &str,
    ) -> Result<(), MailError> {
        self.0.send_with_delivery_id(message, delivery_id).await
    }

    async fn send_for_tenant(&self, tenant_id: &str, message: &Message) -> Result<(), MailError> {
        self.0.send_for_tenant(tenant_id, message).await
    }
}
