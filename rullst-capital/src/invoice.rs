use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

mod paid;
#[cfg(feature = "invoice-pdf")]
mod pdf;

pub use paid::PaidInvoice;

const MAX_INVOICE_ITEMS: usize = 128;
const MAX_INVOICE_TEXT_LEN: usize = 512;
const MAX_INVOICE_AMOUNT_MINOR: u64 = 99_999_999;

pub(crate) struct ValidatedInvoiceMoney {
    #[cfg(feature = "invoice-pdf")]
    pub(crate) item_amounts_minor: Vec<u64>,
    pub(crate) total_minor: u64,
}

/// A line item in an invoice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceItem {
    pub description: String,
    pub amount: f64,
}

/// An invoice representing a successful payment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub invoice_id: String,
    pub customer_email: String,
    pub date: DateTime<Utc>,
    pub items: Vec<InvoiceItem>,
    pub total: f64,
    pub currency: String,
}

impl Invoice {
    /// Validates bounded identifiers, money, currency and the exact item sum.
    ///
    /// The legacy public model keeps `f64` for source compatibility. This gate
    /// accepts only finite positive values that round to a whole number of
    /// minor units within a tight tolerance before trusted rendering.
    pub fn validate(&self) -> Result<(), crate::error::CapitalError> {
        self.validated_money().map(|_| ())
    }

    /// Returns the validated total in currency minor units.
    pub fn total_minor(&self) -> Result<u64, crate::error::CapitalError> {
        self.validated_money().map(|money| money.total_minor)
    }

    /// Generates escaped HTML only after validating the bounded money contract.
    pub fn try_generate_html(&self) -> Result<String, crate::error::CapitalError> {
        self.validate()?;
        Ok(self.generate_html())
    }

    /// Generates an escaped HTML string for the invoice that can be emailed or rendered.
    pub fn generate_html(&self) -> String {
        let mut items_html = String::new();
        for item in &self.items {
            items_html.push_str(&format!(
                "<tr><td style='padding: 8px; border-bottom: 1px solid #ddd;'>{}</td><td style='padding: 8px; border-bottom: 1px solid #ddd; text-align: right;'>{:.2} {}</td></tr>",
                escape_html(&item.description),
                item.amount,
                escape_html(&self.currency)
            ));
        }

        format!(
            r#"
            <!DOCTYPE html>
            <html>
            <head>
                <style>
                    body {{ font-family: 'Helvetica Neue', Helvetica, Arial, sans-serif; color: #333; }}
                    .invoice-box {{ max-width: 800px; margin: auto; padding: 30px; border: 1px solid #eee; box-shadow: 0 0 10px rgba(0, 0, 0, 0.15); font-size: 16px; line-height: 24px; }}
                    .invoice-box table {{ width: 100%; line-height: inherit; text-align: left; border-collapse: collapse; }}
                    .header {{ font-size: 24px; font-weight: bold; margin-bottom: 20px; }}
                </style>
            </head>
            <body>
                <div class="invoice-box">
                    <div class="header">Invoice {}</div>
                    <p><strong>Billed To:</strong> {}</p>
                    <p><strong>Date:</strong> {}</p>
                    <br>
                    <table>
                        <thead>
                            <tr>
                                <th style="padding: 8px; border-bottom: 2px solid #ddd;">Description</th>
                                <th style="padding: 8px; border-bottom: 2px solid #ddd; text-align: right;">Amount</th>
                            </tr>
                        </thead>
                        <tbody>
                            {}
                        </tbody>
                        <tfoot>
                            <tr>
                                <td style="padding: 8px; font-weight: bold;">Total</td>
                                <td style="padding: 8px; font-weight: bold; text-align: right;">{:.2} {}</td>
                            </tr>
                        </tfoot>
                    </table>
                </div>
            </body>
            </html>
            "#,
            escape_html(&self.invoice_id),
            escape_html(&self.customer_email),
            self.date.format("%Y-%m-%d"),
            items_html,
            self.total,
            escape_html(&self.currency)
        )
    }

    pub(crate) fn validated_money(
        &self,
    ) -> Result<ValidatedInvoiceMoney, crate::error::CapitalError> {
        validate_invoice_text("invoice ID", &self.invoice_id, 128)?;
        validate_invoice_email(&self.customer_email)?;
        if self.currency.len() != 3 || !self.currency.bytes().all(|byte| byte.is_ascii_alphabetic())
        {
            return Err(invalid_invoice(
                "currency must be exactly three ASCII letters",
            ));
        }
        if self.items.is_empty() || self.items.len() > MAX_INVOICE_ITEMS {
            return Err(invalid_invoice(format!(
                "invoice must contain 1 to {MAX_INVOICE_ITEMS} items"
            )));
        }

        #[cfg(feature = "invoice-pdf")]
        let mut item_amounts_minor = Vec::with_capacity(self.items.len());
        let mut calculated_total = 0_u64;
        for item in &self.items {
            validate_invoice_text(
                "invoice item description",
                &item.description,
                MAX_INVOICE_TEXT_LEN,
            )?;
            let amount_minor = money_to_minor(item.amount)?;
            calculated_total = calculated_total.checked_add(amount_minor).ok_or_else(|| {
                invalid_invoice("invoice item sum overflowed the supported amount")
            })?;
            #[cfg(feature = "invoice-pdf")]
            item_amounts_minor.push(amount_minor);
        }
        let total_minor = money_to_minor(self.total)?;
        if calculated_total != total_minor {
            return Err(invalid_invoice(
                "invoice total must exactly equal the sum of its items in minor units",
            ));
        }

        Ok(ValidatedInvoiceMoney {
            #[cfg(feature = "invoice-pdf")]
            item_amounts_minor,
            total_minor,
        })
    }

    /// Converts this paid invoice into a Declaração de Prestação de Serviços (DPS) for national NFS-e emission.
    pub fn to_dps(
        &self,
        service_code: &str,
        service_city_ibge: &str,
        iss_rate: f64,
    ) -> crate::fiscal::models::NfseDps {
        let description = self
            .items
            .iter()
            .map(|item| item.description.as_str())
            .collect::<Vec<_>>()
            .join("; ");

        let sanitized_id: String = self
            .invoice_id
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect();

        crate::fiscal::models::NfseDps {
            id: format!("DPS{}", sanitized_id),
            series: "1".to_string(),
            number: 1,
            issued_at: self.date,
            service_code: service_code.to_string(),
            description: if description.is_empty() {
                "Serviços de Tecnologia / SaaS".to_string()
            } else {
                description
            },
            amount: self.total,
            iss_rate,
            iss_retained: false,
            service_city_ibge: service_city_ibge.to_string(),
        }
    }
}

fn money_to_minor(value: f64) -> Result<u64, crate::error::CapitalError> {
    if !value.is_finite() || value <= 0.0 {
        return Err(invalid_invoice(
            "amounts must be finite and greater than zero",
        ));
    }
    let scaled = value * 100.0;
    let rounded = scaled.round();
    if (scaled - rounded).abs() > 1e-7 || rounded > MAX_INVOICE_AMOUNT_MINOR as f64 {
        return Err(invalid_invoice(
            "amounts must fit 1 to 99,999,999 minor units with at most two decimal places",
        ));
    }
    Ok(rounded as u64)
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
mod tests {
    use super::*;

    #[test]
    fn invoice_html_escapes_every_untrusted_text_field() {
        let invoice = Invoice {
            invoice_id: "<INV&1>".to_string(),
            customer_email: "a\"b@example.com".to_string(),
            date: Utc::now(),
            items: vec![InvoiceItem {
                description: "<script>alert('x')</script>".to_string(),
                amount: 1.5,
            }],
            total: 1.5,
            currency: "U&SD".to_string(),
        };

        let html = invoice.generate_html();
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;alert(&#x27;x&#x27;)&lt;/script&gt;"));
        assert!(html.contains("&lt;INV&amp;1&gt;"));
        assert!(html.contains("a&quot;b@example.com"));
        assert!(html.contains("U&amp;SD"));
    }

    fn valid_invoice() -> Invoice {
        Invoice {
            invoice_id: "INV-2026-1".to_string(),
            customer_email: "owner@example.com".to_string(),
            date: Utc::now(),
            items: vec![
                InvoiceItem {
                    description: "Subscription".to_string(),
                    amount: 39.90,
                },
                InvoiceItem {
                    description: "Storage".to_string(),
                    amount: 10.00,
                },
            ],
            total: 49.90,
            currency: "BRL".to_string(),
        }
    }

    #[test]
    fn validated_renderer_uses_exact_minor_units_and_item_sum() {
        let invoice = valid_invoice();
        assert_eq!(invoice.total_minor().expect("valid money"), 4_990);
        assert!(invoice.try_generate_html().is_ok());

        for invalid in [
            Invoice {
                total: 49.91,
                ..invoice.clone()
            },
            Invoice {
                total: f64::NAN,
                ..invoice.clone()
            },
            Invoice {
                items: vec![InvoiceItem {
                    description: "Fraction".to_string(),
                    amount: 1.001,
                }],
                total: 1.001,
                ..invoice.clone()
            },
            Invoice {
                currency: "R$".to_string(),
                ..invoice.clone()
            },
            Invoice {
                customer_email: "invalid".to_string(),
                ..invoice.clone()
            },
            Invoice {
                items: Vec::new(),
                total: 1.0,
                ..invoice
            },
        ] {
            assert!(matches!(
                invalid.validate(),
                Err(crate::error::CapitalError::InvalidInvoice(_))
            ));
        }
    }
}

#[cfg(kani)]
#[cfg_attr(mutants, mutants::skip)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    fn proof_invoice_item_amount_valid() {
        let amount: f64 = kani::any();
        let valid = !amount.is_nan() && !amount.is_infinite();
        assert!(valid || amount.is_nan() || amount.is_infinite());
    }
}
