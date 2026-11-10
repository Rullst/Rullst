use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

mod labels;
mod paid;
#[cfg(feature = "invoice-pdf")]
mod pdf;

pub use labels::{InvoiceLabels, InvoiceLocale};
pub use paid::PaidInvoice;

const MAX_INVOICE_ITEMS: usize = 128;
const MAX_INVOICE_TEXT_LEN: usize = 512;
const MAX_INVOICE_AMOUNT_MINOR: u64 = 99_999_999;

/// A line item in an invoice.
///
/// Since 13.0.0-alpha.5 the amount is an integer in the currency's smallest
/// unit (cents for BRL or USD, whole yen for JPY), like
/// [`ChargeReceipt::amount_minor`](crate::ChargeReceipt::amount_minor).
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceItem {
    /// The line shown to the customer.
    pub description: String,
    /// Amount in the invoice currency's minor units (1 to 99,999,999).
    pub amount_minor: u64,
}

impl InvoiceItem {
    /// Creates a line item; [`Invoice::validate`] checks the bounds.
    pub fn new(description: impl Into<String>, amount_minor: u64) -> Self {
        Self {
            description: description.into(),
            amount_minor,
        }
    }
}

/// An invoice representing a successful payment.
///
/// Since 13.0.0-alpha.5 amounts are integer minor units and the total is the
/// sum of the items ([`Self::total_minor`]); build one with [`Self::new`] and
/// [`Self::with_item`]. The HTML and PDF print [`Self::labels`], English by
/// default ([`Self::with_locale`]).
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    /// Application invoice identifier (1 to 128 characters).
    pub invoice_id: String,
    /// The recipient, which must equal the charge receipt's e-mail address.
    pub customer_email: String,
    /// Issue date, printed as `YYYY-MM-DD`.
    pub date: DateTime<Utc>,
    /// One to 128 line items.
    pub items: Vec<InvoiceItem>,
    /// Three-letter ISO 4217 currency code of every amount.
    pub currency: String,
    /// Words printed on the HTML and PDF; English when absent.
    #[serde(default)]
    pub labels: InvoiceLabels,
}

impl Invoice {
    /// Creates an invoice without items and with English labels.
    pub fn new(
        invoice_id: impl Into<String>,
        customer_email: impl Into<String>,
        date: DateTime<Utc>,
        currency: impl Into<String>,
    ) -> Self {
        Self {
            invoice_id: invoice_id.into(),
            customer_email: customer_email.into(),
            date,
            items: Vec::new(),
            currency: currency.into(),
            labels: InvoiceLabels::default(),
        }
    }

    /// Appends a line item of `amount_minor` currency minor units.
    pub fn with_item(mut self, description: impl Into<String>, amount_minor: u64) -> Self {
        self.items.push(InvoiceItem::new(description, amount_minor));
        self
    }

    /// Prints the labels of a built-in locale.
    pub fn with_locale(mut self, locale: InvoiceLocale) -> Self {
        self.labels = locale.labels();
        self
    }

    /// Prints custom labels, for another language or wording.
    pub fn with_labels(mut self, labels: InvoiceLabels) -> Self {
        self.labels = labels;
        self
    }

    /// Validates bounded identifiers, labels, currency and each amount
    /// (1 to 99,999,999 minor units, and a total within that bound).
    pub fn validate(&self) -> Result<(), crate::error::CapitalError> {
        self.validated_total().map(|_| ())
    }

    /// Returns the validated total, the sum of the items in currency minor units.
    pub fn total_minor(&self) -> Result<u64, crate::error::CapitalError> {
        self.validated_total()
    }

    /// Generates escaped HTML only after validating the invoice.
    pub fn try_generate_html(&self) -> Result<String, crate::error::CapitalError> {
        self.validate()?;
        Ok(self.generate_html())
    }

    /// Generates an escaped HTML string for the invoice that can be emailed or rendered.
    ///
    /// It does not validate the invoice; prefer [`Self::try_generate_html`].
    /// Amounts use the currency's ISO 4217 minor-unit decimals (for example
    /// none for JPY and three for KWD).
    pub fn generate_html(&self) -> String {
        let labels = &self.labels;
        let currency = self.currency.to_ascii_uppercase();
        let mut items_html = String::new();
        for item in &self.items {
            items_html.push_str(&format!(
                "<tr><td style='padding: 8px; border-bottom: 1px solid #ddd;'>{}</td><td style='padding: 8px; border-bottom: 1px solid #ddd; text-align: right;'>{}</td></tr>",
                escape_html(&item.description),
                escape_html(&format_money(item.amount_minor, &currency)),
            ));
        }
        let total = self
            .items
            .iter()
            .fold(0_u64, |sum, item| sum.saturating_add(item.amount_minor));

        format!(
            r#"
            <!DOCTYPE html>
            <html lang="{}">
            <head>
                <meta charset="utf-8">
                <style>
                    body {{ font-family: 'Helvetica Neue', Helvetica, Arial, sans-serif; color: #333; }}
                    .invoice-box {{ max-width: 800px; margin: auto; padding: 30px; border: 1px solid #eee; box-shadow: 0 0 10px rgba(0, 0, 0, 0.15); font-size: 16px; line-height: 24px; }}
                    .invoice-box table {{ width: 100%; line-height: inherit; text-align: left; border-collapse: collapse; }}
                    .header {{ font-size: 24px; font-weight: bold; margin-bottom: 20px; }}
                </style>
            </head>
            <body>
                <div class="invoice-box">
                    <div class="header">{} {}</div>
                    <p><strong>{}:</strong> {}</p>
                    <p><strong>{}:</strong> {}</p>
                    <br>
                    <table>
                        <thead>
                            <tr>
                                <th style="padding: 8px; border-bottom: 2px solid #ddd;">{}</th>
                                <th style="padding: 8px; border-bottom: 2px solid #ddd; text-align: right;">{}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {}
                        </tbody>
                        <tfoot>
                            <tr>
                                <td style="padding: 8px; font-weight: bold;">{}</td>
                                <td style="padding: 8px; font-weight: bold; text-align: right;">{}</td>
                            </tr>
                        </tfoot>
                    </table>
                </div>
            </body>
            </html>
            "#,
            escape_html(&labels.language),
            escape_html(&labels.title),
            escape_html(&self.invoice_id),
            escape_html(&labels.billed_to),
            escape_html(&self.customer_email),
            escape_html(&labels.date),
            self.date.format("%Y-%m-%d"),
            escape_html(&labels.description),
            escape_html(&labels.amount),
            items_html,
            escape_html(&labels.total),
            escape_html(&format_money(total, &currency)),
        )
    }

    pub(crate) fn validated_total(&self) -> Result<u64, crate::error::CapitalError> {
        validate_invoice_text("invoice ID", &self.invoice_id, 128)?;
        validate_invoice_email(&self.customer_email)?;
        if self.currency.len() != 3 || !self.currency.bytes().all(|byte| byte.is_ascii_alphabetic())
        {
            return Err(invalid_invoice(
                "currency must be exactly three ASCII letters",
            ));
        }
        self.labels.validate()?;
        if self.items.is_empty() || self.items.len() > MAX_INVOICE_ITEMS {
            return Err(invalid_invoice(format!(
                "invoice must contain 1 to {MAX_INVOICE_ITEMS} items"
            )));
        }

        let mut total = 0_u64;
        for item in &self.items {
            validate_invoice_text(
                "invoice item description",
                &item.description,
                MAX_INVOICE_TEXT_LEN,
            )?;
            if !(1..=MAX_INVOICE_AMOUNT_MINOR).contains(&item.amount_minor) {
                return Err(invalid_invoice(
                    "item amounts must contain 1 to 99,999,999 currency minor units",
                ));
            }
            total = total.saturating_add(item.amount_minor);
        }
        if total > MAX_INVOICE_AMOUNT_MINOR {
            return Err(invalid_invoice(
                "invoice total must not exceed 99,999,999 currency minor units",
            ));
        }
        Ok(total)
    }
}

/// Formats minor units with the currency's ISO 4217 exponent:
/// `49.90 BRL`, `2500 JPY`, `12.345 KWD`.
pub(crate) fn format_money(amount_minor: u64, currency: &str) -> String {
    let exponent = crate::currency::minor_unit_exponent(currency);
    if exponent == 0 {
        return format!("{amount_minor} {currency}");
    }
    let scale = 10_u64.pow(exponent);
    format!(
        "{}.{:0width$} {currency}",
        amount_minor / scale,
        amount_minor % scale,
        width = exponent as usize
    )
}

fn validate_invoice_text(
    label: &str,
    value: &str,
    max_len: usize,
) -> Result<(), crate::error::CapitalError> {
    if value.is_empty()
        || value.len() > max_len
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        return Err(invalid_invoice(format!(
            "{label} must contain 1 to {max_len} non-control bytes without surrounding whitespace"
        )));
    }
    Ok(())
}

fn validate_invoice_email(value: &str) -> Result<(), crate::error::CapitalError> {
    validate_invoice_text("customer email", value, 320)?;
    let mut parts = value.split('@');
    if parts.next().is_none_or(str::is_empty)
        || parts.next().is_none_or(str::is_empty)
        || parts.next().is_some()
        || value.chars().any(char::is_whitespace)
    {
        return Err(invalid_invoice(
            "customer email must contain one non-empty local and domain part",
        ));
    }
    Ok(())
}

fn invalid_invoice(message: impl Into<String>) -> crate::error::CapitalError {
    crate::error::CapitalError::InvalidInvoice(message.into())
}

fn escape_html(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#x27;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

#[cfg(test)]
#[path = "invoice_tests.rs"]
mod tests;

#[cfg(kani)]
#[cfg_attr(mutants, mutants::skip)]
mod kani_proofs {
    use super::*;

    /// Every accepted item amount, summed over the largest invoice, fits a
    /// `u64`, so the validated total is exact.
    #[kani::proof]
    fn proof_invoice_item_amount_valid() {
        let amount: u64 = kani::any();
        kani::assume((1..=MAX_INVOICE_AMOUNT_MINOR).contains(&amount));
        assert!(amount.checked_mul(MAX_INVOICE_ITEMS as u64).is_some());
    }
}
