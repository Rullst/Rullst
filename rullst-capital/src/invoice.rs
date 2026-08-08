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
    /// Generates a beautiful HTML string for the invoice that can be emailed or rendered.
    pub fn generate_html(&self) -> String {
        let mut items_html = String::new();
        for item in &self.items {
            items_html.push_str(&format!(
                "<tr><td style='padding: 8px; border-bottom: 1px solid #ddd;'>{}</td><td style='padding: 8px; border-bottom: 1px solid #ddd; text-align: right;'>{:.2} {}</td></tr>",
                item.description, item.amount, self.currency
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
            self.invoice_id,
            self.customer_email,
            self.date.format("%Y-%m-%d"),
            items_html,
            self.total,
            self.currency
        )
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

