//! Evidence-aware fiscal and payment-recovery mailable templates.

pub(super) const FISCAL_INVOICE_TEMPLATE: &str = r##"//! Evidence-aware NFS-e and international receipt mailable.
use rullst::capital::fiscal::{FiscalResponse, FiscalResponseKind};
use rullst::mail::{
    DeliveryPipeline, Mail, MailError, Message, escape_html, validate_action_url,
};

#[derive(Debug, Clone)]
enum InvoiceEvidence {
    NfseOfficial { number: u64, access_key: String },
    NfseOfflinePreview,
    InternationalReceipt { receipt_id: String },
}

/// Fiscal/receipt notification which keeps typed offline or contradictory markers unauthorized.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct __NAME__ {
    to: String,
    customer_name: String,
    amount: String,
    document_url: Option<String>,
    evidence: InvoiceEvidence,
}

impl __NAME__ {
    /// Builds an NFS-e notification from the typed fiscal provenance returned by Capital.
    pub fn from_nfse_response(
        to: impl Into<String>,
        customer_name: impl Into<String>,
        amount: impl Into<String>,
        response: &FiscalResponse,
    ) -> Result<Self, MailError> {
        let evidence = if response.is_officially_authorized() {
            if response.nfse_number == 0
                || response.access_key.trim().is_empty()
                || response.access_key.starts_with("MOCK-")
            {
                return Err(validation("official NFS-e evidence is incomplete"));
            }
            InvoiceEvidence::NfseOfficial {
                number: response.nfse_number,
                access_key: bounded("NFS-e access key", &response.access_key, 128)?,
            }
        } else if response.kind == FiscalResponseKind::OfflineMock
            && response.nfse_number == 0
            && response.access_key.starts_with("MOCK-NOT-AUTHORIZED-")
        {
            InvoiceEvidence::NfseOfflinePreview
        } else {
            return Err(validation("unrecognized or contradictory NFS-e provenance"));
        };

        Ok(Self {
            to: bounded("recipient", to, 320)?,
            customer_name: bounded("customer name", customer_name, 150)?,
            amount: bounded("amount", amount, 64)?,
            document_url: None,
            evidence,
        })
    }

    /// Builds a commercial cross-border receipt without presenting it as a tax authorization.
    pub fn international_receipt(
        to: impl Into<String>,
        customer_name: impl Into<String>,
        receipt_id: impl Into<String>,
        amount: impl Into<String>,
    ) -> Result<Self, MailError> {
        Ok(Self {
            to: bounded("recipient", to, 320)?,
            customer_name: bounded("customer name", customer_name, 150)?,
            amount: bounded("amount", amount, 64)?,
            document_url: None,
            evidence: InvoiceEvidence::InternationalReceipt {
                receipt_id: bounded("receipt ID", receipt_id, 128)?,
            },
        })
    }

    /// Adds the application-owned HTTPS/HTTP document location.
    pub fn with_document_url(mut self, url: impl Into<String>) -> Result<Self, MailError> {
        let url = bounded("document URL", url, 2_048)?;
        validate_action_url(&url)?;
        self.document_url = Some(url);
        Ok(self)
    }

    /// Builds and runs the mandatory mail pre-flight before returning the message.
    pub fn build(&self) -> Result<Message, MailError> {
        let customer_name = escape_html(&self.customer_name);
        let amount = escape_html(&self.amount);
        let (subject, heading, badge, explanation, identifier) = match &self.evidence {
            InvoiceEvidence::NfseOfficial { number, access_key } => (
                format!("NFS-e #{number} issued"),
                "NFS-e issued",
                "OFFICIALLY AUTHORIZED",
                "The application supplied a response marked as an official tax-authority authorization.",
                format!("NFS-e #{number} · access key {}", escape_html(access_key)),
            ),
            InvoiceEvidence::NfseOfflinePreview => (
                "[PREVIEW — NOT AUTHORIZED] NFS-e DPS".to_string(),
                "NFS-e preview",
                "NOT AUTHORIZED",
                "This is an offline development preview. No tax authority received or authorized it.",
                "DPS preview only".to_string(),
            ),
            InvoiceEvidence::InternationalReceipt { receipt_id } => (
                format!("Payment receipt {}", receipt_id),
                "Payment receipt",
                "COMMERCIAL RECEIPT",
                "This receipt records an application payment and is not a tax authorization.",
                format!("Receipt {}", escape_html(receipt_id)),
            ),
        };
        let document_link = match self.document_url.as_deref() {
            Some(url) => format!(
                r#"<p><a href="{}" style="color:#93c5fd">Open the application document</a></p>"#,
                escape_html(url)
            ),
            None => String::new(),
        };
        let html = format!(
            r#"<!doctype html>
<html><head><meta charset="utf-8"><title>{}</title></head>
<body style="font-family:sans-serif;background:#030712;color:#f8fafc;padding:32px 16px">
  <main style="max-width:600px;margin:auto;background:#111827;border:1px solid #334155;border-radius:12px;padding:28px">
    <p style="color:#fbbf24;font-weight:800;letter-spacing:.08em">{}</p>
    <h1 style="font-size:24px">{}</h1>
    <p>Hello {},</p>
    <p>{}</p>
    <section style="background:#0f172a;border-radius:8px;padding:18px;margin:20px 0">
      <strong>{}</strong><br><span style="font-size:24px">{}</span>
    </section>
    {}
  </main>
</body></html>"#,
            escape_html(&subject), badge, heading, customer_name, explanation, identifier, amount,
            document_link
        );
        let message = Message::new()
            .to(&self.to)
            .subject(subject)
            .html(html)
            .sanitize_secrets();
        DeliveryPipeline::prepare(&message).map(|prepared| prepared.into_message())
    }

    pub async fn send(&self) -> Result<(), MailError> {
        Mail::send(self.build()?).await
    }
}

fn bounded(
    field: &'static str,
    value: impl Into<String>,
    maximum: usize,
) -> Result<String, MailError> {
    let value = value.into();
    let length = value.chars().count();
    if length == 0 || length > maximum {
        return Err(validation(&format!(
            "{field} must contain between 1 and {maximum} characters"
        )));
    }
    Ok(value)
}

fn validation(reason: &str) -> MailError {
    MailError::ValidationError(reason.to_string())
}
"##;

pub(super) const DUNNING_TEMPLATE: &str = r##"//! Explicit progressive payment-recovery mailable.
use rullst::mail::{
    DeliveryPipeline, Mail, MailError, Message, escape_html, validate_action_url,
};

/// Application-confirmed point in a D+1/D+3/D+7 payment-recovery sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DunningStage {
    GentleReminder,
    ActionRequired,
    ServicePaused,
}

impl DunningStage {
    #[must_use]
    pub const fn days_after_due(self) -> u8 {
        match self {
            Self::GentleReminder => 1,
            Self::ActionRequired => 3,
            Self::ServicePaused => 7,
        }
    }
}

/// Deterministic dunning message; scheduling and account state remain application-owned.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct __NAME__ {
    to: String,
    customer_name: String,
    invoice_id: String,
    amount_due: String,
    stage: DunningStage,
    billing_url: Option<String>,
}

impl __NAME__ {
    pub fn new(
        to: impl Into<String>,
        customer_name: impl Into<String>,
        invoice_id: impl Into<String>,
        amount_due: impl Into<String>,
        stage: DunningStage,
    ) -> Result<Self, MailError> {
        Ok(Self {
            to: bounded("recipient", to, 320)?,
            customer_name: bounded("customer name", customer_name, 150)?,
            invoice_id: bounded("invoice ID", invoice_id, 128)?,
            amount_due: bounded("amount due", amount_due, 64)?,
            stage,
            billing_url: None,
        })
    }

    /// Adds the application-owned payment-management URL.
    pub fn with_billing_url(mut self, url: impl Into<String>) -> Result<Self, MailError> {
        let url = bounded("billing URL", url, 2_048)?;
        validate_action_url(&url)?;
        self.billing_url = Some(url);
        Ok(self)
    }

    /// Builds and runs the mandatory mail pre-flight before returning the message.
    pub fn build(&self) -> Result<Message, MailError> {
        let (subject, heading, explanation, color) = match self.stage {
            DunningStage::GentleReminder => (
                format!("Payment reminder for invoice {}", self.invoice_id),
                "A gentle payment reminder",
                "Our records show this invoice is one day past due. If payment is already processing, no action is needed.",
                "#60a5fa",
            ),
            DunningStage::ActionRequired => (
                format!("Action required for invoice {}", self.invoice_id),
                "Payment action required",
                "This invoice is three days past due. Please review the payment method or contact support.",
                "#f59e0b",
            ),
            DunningStage::ServicePaused => (
                format!("Service status for invoice {}", self.invoice_id),
                "Payment remains unresolved",
                "This invoice is seven days past due. Access may be paused only according to your application's disclosed billing policy.",
                "#ef4444",
            ),
        };
        let action = match self.billing_url.as_deref() {
            Some(url) => format!(
                r#"<p><a href="{}" style="color:#93c5fd">Review billing details</a></p>"#,
                escape_html(url)
            ),
            None => String::new(),
        };
        let html = format!(
            r#"<!doctype html>
<html><head><meta charset="utf-8"><title>{}</title></head>
<body style="font-family:sans-serif;background:#030712;color:#f8fafc;padding:32px 16px">
  <main style="max-width:600px;margin:auto;background:#111827;border:1px solid #334155;border-radius:12px;padding:28px">
    <p style="color:{};font-weight:800">D+{} PAYMENT RECOVERY</p>
    <h1>{}</h1>
    <p>Hello {},</p>
    <p>{}</p>
    <section style="background:#0f172a;border-radius:8px;padding:18px;margin:20px 0">
      <strong>Invoice {}</strong><br><span style="font-size:24px">{}</span>
    </section>
    {}
  </main>
</body></html>"#,
            escape_html(&subject), color, self.stage.days_after_due(), heading,
            escape_html(&self.customer_name), explanation, escape_html(&self.invoice_id),
            escape_html(&self.amount_due), action
        );
        let message = Message::new()
            .to(&self.to)
            .subject(subject)
            .html(html)
            .sanitize_secrets();
        DeliveryPipeline::prepare(&message).map(|prepared| prepared.into_message())
    }

    pub async fn send(&self) -> Result<(), MailError> {
        Mail::send(self.build()?).await
    }
}

fn bounded(
    field: &'static str,
    value: impl Into<String>,
    maximum: usize,
) -> Result<String, MailError> {
    let value = value.into();
    let length = value.chars().count();
    if length == 0 || length > maximum {
        return Err(MailError::ValidationError(format!(
            "{field} must contain between 1 and {maximum} characters"
        )));
    }
    Ok(value)
}
"##;
