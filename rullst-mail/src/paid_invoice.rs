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
    ///
    /// The subject is the invoice's title label and ID (`Invoice INV-42`, or
    /// `Recibo INV-42` for an invoice with Portuguese or Spanish labels) and
    /// the PDF is named after the title (`invoice-<key>.pdf`,
    /// `recibo-<key>.pdf`); [`Self::subject`] and [`Self::filename`] replace
    /// them.
    pub fn prepare(paid: &PaidInvoice) -> Result<Self, MailError> {
        let invoice = paid.invoice();
        let html = invoice.try_generate_html().map_err(capital_error)?;
        let pdf = invoice.generate_pdf().map_err(capital_error)?;
        let filename = format!(
            "{}-{}.pdf",
            file_stem(&invoice.labels.title),
            short_key(paid.delivery_key())?
        );
        let message = Message::new()
            .to(&invoice.customer_email)
            .subject(format!("{} {}", invoice.labels.title, invoice.invoice_id))
            .html(html)
            .attach_bytes(filename, pdf, "application/pdf");
        let prepared = DeliveryPipeline::prepare(&message)?;
        Ok(Self {
            delivery_key: paid.delivery_key().to_string(),
            prepared,
        })
    }

    /// Sets the application-owned verified sender and re-runs the pre-flight
    /// pipeline, which rejects a header-breaking or malformed address.
    ///
    /// Real providers require a sender from a verified domain; without one,
    /// some drivers fail with a configuration error and others fall back to a
    /// placeholder address that providers reject. New in 13.0.
    pub fn from(self, sender: impl Into<String>) -> Result<Self, MailError> {
        let mut message = self.prepared.into_message();
        message.from = Some(sender.into());
        Ok(Self {
            delivery_key: self.delivery_key,
            prepared: DeliveryPipeline::prepare(&message)?,
        })
    }

    /// Replaces the subject and re-runs the pre-flight pipeline, which
    /// rejects a header-breaking value. Since 13.0.0-alpha.5.
    pub fn subject(self, subject: impl Into<String>) -> Result<Self, MailError> {
        let mut message = self.prepared.into_message();
        message.subject = subject.into();
        Ok(Self {
            delivery_key: self.delivery_key,
            prepared: DeliveryPipeline::prepare(&message)?,
        })
    }

    /// Replaces the PDF attachment's file name, a basename ending in `.pdf`,
    /// and re-runs the pre-flight pipeline. Since 13.0.0-alpha.5.
    pub fn filename(self, filename: impl Into<String>) -> Result<Self, MailError> {
        let filename = filename.into();
        if !filename
            .get(filename.len().saturating_sub(4)..)
            .is_some_and(|extension| extension.eq_ignore_ascii_case(".pdf"))
        {
            return Err(MailError::ValidationError(
                "paid invoice file name must end in .pdf".to_string(),
            ));
        }
        let mut message = self.prepared.into_message();
        for attachment in &mut message.attachments {
            attachment.filename.clone_from(&filename);
        }
        Ok(Self {
            delivery_key: self.delivery_key,
            prepared: DeliveryPipeline::prepare(&message)?,
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

/// The lowercase ASCII words of the title (`Recibo` → `recibo`), or `invoice`.
fn file_stem(title: &str) -> String {
    let mut stem = String::with_capacity(title.len());
    for character in title.chars() {
        if character.is_ascii_alphanumeric() {
            stem.push(character.to_ascii_lowercase());
        } else if !stem.is_empty() && !stem.ends_with('-') {
            stem.push('-');
        }
    }
    let stem = stem.trim_end_matches('-');
    if stem.is_empty() {
        "invoice".to_string()
    } else {
        stem.to_string()
    }
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
    use rullst_capital::{ChargeReceipt, ChargeStatus, Invoice, InvoiceLocale};

    fn invoice() -> Invoice {
        Invoice::new("INV-2026-42", "owner@example.com", Utc::now(), "BRL")
            .with_item("Serviço de hospedagem", 4_990)
    }

    fn paid_invoice(status: ChargeStatus) -> Result<PaidInvoice, rullst_capital::CapitalError> {
        bound(invoice(), status)
    }

    fn bound(
        invoice: Invoice,
        status: ChargeStatus,
    ) -> Result<PaidInvoice, rullst_capital::CapitalError> {
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
    fn subject_and_file_name_follow_the_invoice_labels() {
        let english = PaidInvoiceDelivery::prepare(&paid_invoice(ChargeStatus::Succeeded).unwrap())
            .expect("prepared delivery");
        assert_eq!(english.message().subject, "Invoice INV-2026-42");
        let name = &english.message().attachments[0].filename;
        assert!(
            name.starts_with("invoice-") && name.ends_with(".pdf"),
            "{name}"
        );
        assert_eq!(name.len(), "invoice-".len() + 24 + ".pdf".len());

        let receipt = bound(
            invoice().with_locale(InvoiceLocale::PtBr),
            ChargeStatus::Succeeded,
        )
        .expect("paid receipt");
        let delivery = PaidInvoiceDelivery::prepare(&receipt).expect("prepared receipt");
        assert_eq!(delivery.message().subject, "Recibo INV-2026-42");
        assert!(
            delivery.message().attachments[0]
                .filename
                .starts_with("recibo-")
        );
        assert!(
            delivery
                .message()
                .body_html
                .as_deref()
                .is_some_and(|body| body.contains("Recebido de"))
        );
    }

    #[test]
    fn subject_and_file_name_can_be_replaced_and_are_validated() {
        let delivery =
            PaidInvoiceDelivery::prepare(&paid_invoice(ChargeStatus::Succeeded).unwrap())
                .expect("prepared delivery");
        let key = delivery.delivery_key().to_string();
        let changed = delivery
            .clone()
            .subject("Seu recibo da aula")
            .and_then(|delivery| delivery.filename("recibo-aula.PDF"))
            .expect("valid subject and file name");
        assert_eq!(changed.message().subject, "Seu recibo da aula");
        assert_eq!(changed.message().attachments[0].filename, "recibo-aula.PDF");
        assert_eq!(changed.delivery_key(), key);
        assert!(
            changed.message().attachments[0]
                .content
                .starts_with(b"%PDF-")
        );

        assert!(
            delivery
                .clone()
                .subject("Receipt\r\nBcc: other@example.com")
                .is_err()
        );
        for invalid in [
            "receipt.txt",
            "pdf",
            "../receipt.pdf",
            "a/b.pdf",
            ".pdf\u{1}",
        ] {
            assert!(
                matches!(
                    delivery.clone().filename(invalid),
                    Err(MailError::ValidationError(_))
                ),
                "{invalid}"
            );
        }
        assert!(delivery.filename("x.pdf").is_ok());
    }

    #[test]
    fn file_stems_keep_ascii_words_of_the_title() {
        assert_eq!(file_stem("Invoice"), "invoice");
        assert_eq!(file_stem("Recibo"), "recibo");
        assert_eq!(file_stem("Tax Receipt 2026"), "tax-receipt-2026");
        assert_eq!(file_stem("  Nota de débito! "), "nota-de-d-bito");
        assert_eq!(file_stem("Квитанция"), "invoice");
        assert_eq!(file_stem(""), "invoice");
    }

    #[test]
    fn non_final_payment_cannot_reach_the_delivery_builder() {
        for status in [ChargeStatus::Processing, ChargeStatus::Mock] {
            assert!(paid_invoice(status).is_err());
        }
    }

    #[tokio::test]
    async fn verified_sender_is_validated_and_delivered() {
        let paid = paid_invoice(ChargeStatus::Succeeded).expect("paid invoice");
        let delivery = PaidInvoiceDelivery::prepare(&paid).expect("prepared delivery");
        assert!(delivery.message().from.is_none());
        for invalid in [
            "billing@example.com\r\nBcc: other@example.com",
            "not-an-address",
        ] {
            assert!(delivery.clone().from(invalid).is_err());
        }
        let delivery = delivery
            .from("billing@example.com")
            .expect("verified sender");
        assert_eq!(delivery.delivery_key(), paid.delivery_key());
        let (driver, messages) = MemoryDriver::isolated();
        delivery.send_with(&driver).await.expect("offline delivery");
        let messages = messages.lock().expect("message store");
        assert_eq!(messages[0].from.as_deref(), Some("billing@example.com"));
        assert!(messages[0].attachments[0].content.starts_with(b"%PDF-"));
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
