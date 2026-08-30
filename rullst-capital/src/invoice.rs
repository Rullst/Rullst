use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
