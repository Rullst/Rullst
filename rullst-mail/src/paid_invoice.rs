//! Payment-bound invoice delivery with a native PDF attachment.

use crate::{DeliveryPipeline, Mail, MailDriver, MailError, Message, PreparedMessage};
use rullst_capital::PaidInvoice;

/// A pipeline-validated invoice message bound to final payment evidence.
///
/// The stable delivery key lets the application atomically claim its own
/// durable outbox record before calling [`Self::send`]. Rullst Mail is
/// at-least-once and does not claim distributed exactly-once provider delivery.
#[derive(Clone)]
pub struct PaidInvoiceDelivery {
    delivery_key: String,
    prepared: PreparedMessage,
}

impl std::fmt::Debug for PaidInvoiceDelivery {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PaidInvoiceDelivery")
            .field("delivery_key", &self.delivery_key)
            .field("recipient", &"[REDACTED]")
            .field("subject", &self.prepared.message().subject)
            .field(
                "attachment_count",
                &self.prepared.message().attachments.len(),
            )
            .finish()
    }
}

impl PaidInvoiceDelivery {
    /// Renders HTML and PDF, attaches the document and runs the mail pre-flight pipeline.
    pub fn prepare(paid: &PaidInvoice) -> Result<Self, MailError> {
        let invoice = paid.invoice();
        let html = invoice.try_generate_html().map_err(capital_error)?;
        let pdf = invoice.generate_pdf().map_err(capital_error)?;
        let filename = format!("invoice-{}.pdf", short_key(paid.delivery_key())?);
        let message = Message::new()
            .to(&invoice.customer_email)
            .subject(format!("Invoice {}", invoice.invoice_id))
            .html(html)
            .attach_bytes(filename, pdf, "application/pdf");
        let prepared = DeliveryPipeline::prepare(&message)?;
        Ok(Self {
            delivery_key: paid.delivery_key().to_string(),
            prepared,
        })
    }

    /// Stable non-secret key for the application's durable delivery outbox.
    pub fn delivery_key(&self) -> &str {
        &self.delivery_key
    }

    /// Returns the sanitized, pipeline-validated message.
    pub fn message(&self) -> &Message {
        self.prepared.message()
    }

    /// Sends immediately or enqueues through the globally configured Mail facade.
    ///
    /// Persist and atomically claim [`Self::delivery_key`] in a durable outbox
    /// before this call when retries or multiple application instances are possible.
    pub async fn send(&self) -> Result<(), MailError> {
        Mail::send(self.prepared.message().clone()).await
    }

    /// Sends synchronously through an explicit statically dispatched driver.
    pub async fn send_with<D: MailDriver>(&self, driver: &D) -> Result<(), MailError> {
        driver.send(self.prepared.message()).await
    }

    /// Sends or enqueues through the tenant-aware Mail facade.
    pub async fn send_for_tenant(&self, tenant_id: impl Into<String>) -> Result<(), MailError> {
        Mail::send_for_tenant(tenant_id, self.prepared.message().clone()).await
    }
}

fn capital_error(error: rullst_capital::CapitalError) -> MailError {
    MailError::ValidationError(format!("paid invoice could not be rendered: {error}"))
}

fn short_key(value: &str) -> Result<&str, MailError> {
    value
        .strip_prefix("paid_invoice_")
        .and_then(|digest| digest.get(..24))
        .filter(|digest| digest.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or_else(|| {
            MailError::ValidationError("paid invoice delivery key is invalid".to_string())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemoryDriver;
    use chrono::Utc;
    use rullst_capital::{ChargeReceipt, ChargeStatus, Invoice, InvoiceItem};

    fn paid_invoice(status: ChargeStatus) -> Result<PaidInvoice, rullst_capital::CapitalError> {
        let invoice = Invoice {
            invoice_id: "INV-2026-42".to_string(),
            customer_email: "owner@example.com".to_string(),
            date: Utc::now(),
            items: vec![InvoiceItem {
                description: "Serviço de hospedagem".to_string(),
                amount: 49.90,
            }],
            total: 49.90,
            currency: "BRL".to_string(),
        };
        let receipt = ChargeReceipt::from_verified_provider_response(
            "fixture",
            "charge_42",
            status,
            4_990,
            "brl",
            "owner@example.com",
        )?;
        invoice.bind_succeeded_charge(&receipt)
    }

    #[test]
    fn prepares_validated_html_and_native_pdf_from_final_payment() {
        let paid = paid_invoice(ChargeStatus::Succeeded).expect("paid invoice");
        let delivery = PaidInvoiceDelivery::prepare(&paid).expect("prepared delivery");
        assert_eq!(delivery.delivery_key(), paid.delivery_key());
        assert_eq!(delivery.message().to, "owner@example.com");
        assert_eq!(delivery.message().attachments.len(), 1);
        let attachment = &delivery.message().attachments[0];
        assert_eq!(attachment.mime_type, "application/pdf");
        assert!(attachment.filename.starts_with("invoice-"));
        assert!(attachment.content.starts_with(b"%PDF-"));
        assert!(
            delivery
                .message()
                .body_html
                .as_deref()
                .is_some_and(|body| body.contains("Serviço de hospedagem"))
        );
        assert!(!format!("{delivery:?}").contains("owner@example.com"));
        assert!(!format!("{delivery:?}").contains("%PDF-"));
    }

    #[test]
    fn non_final_payment_cannot_reach_the_delivery_builder() {
        for status in [ChargeStatus::Processing, ChargeStatus::Mock] {
            assert!(paid_invoice(status).is_err());
        }
    }

    #[tokio::test]
    async fn explicit_driver_receives_the_pdf_delivery() {
        let paid = paid_invoice(ChargeStatus::Succeeded).expect("paid invoice");
        let delivery = PaidInvoiceDelivery::prepare(&paid).expect("prepared delivery");
        let (driver, messages) = MemoryDriver::isolated();
        delivery.send_with(&driver).await.expect("offline delivery");
        let messages = messages.lock().expect("message store");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].attachments.len(), 1);
        assert!(messages[0].attachments[0].content.starts_with(b"%PDF-"));
    }
}
