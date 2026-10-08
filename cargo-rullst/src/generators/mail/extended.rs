//! Payment-receipt and payment-recovery mailable templates.

pub(super) const PAYMENT_RECEIPT_TEMPLATE: &str = r##"//! Bounded commercial payment-receipt mailable.
use rullst::mail::{
    DeliveryPipeline, Mail, MailError, Message, escape_html, validate_action_url,
};

/// Payment receipt that never presents itself as a tax authorization.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct __NAME__ {
    to: String,
    customer_name: String,
    amount: String,
    receipt_id: String,
    document_url: Option<String>,
}

impl __NAME__ {
    /// Builds a commercial receipt for a payment the application already confirmed.
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
            receipt_id: bounded("receipt ID", receipt_id, 128)?,
            document_url: None,
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
        let subject = format!("Payment receipt {}", self.receipt_id);
        let identifier = format!("Receipt {}", escape_html(&self.receipt_id));
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
    <p style="color:#fbbf24;font-weight:800;letter-spacing:.08em">COMMERCIAL RECEIPT</p>
    <h1 style="font-size:24px">Payment receipt</h1>
    <p>Hello {},</p>
    <p>This receipt records an application payment and is not a tax authorization.</p>
    <section style="background:#0f172a;border-radius:8px;padding:18px;margin:20px 0">
      <strong>{}</strong><br><span style="font-size:24px">{}</span>
    </section>
    {}
  </main>
</body></html>"#,
            escape_html(&subject), customer_name, identifier, amount, document_link
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
