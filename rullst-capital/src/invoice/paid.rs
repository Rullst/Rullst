use super::{Invoice, invalid_invoice};
use crate::{ChargeReceipt, ChargeStatus};
use ring::digest::{SHA256, digest};

/// An immutable invoice bound to one final successful charge receipt.
///
/// Construction validates the invoice and requires exact recipient, amount and
/// currency agreement. The billing adapter remains responsible for verifying
/// its provider response before constructing the receipt.
#[derive(Clone)]
pub struct PaidInvoice {
    invoice: Invoice,
    provider: &'static str,
    charge_id: String,
    delivery_key: String,
}

impl PaidInvoice {
    /// Returns the validated invoice payload.
    pub fn invoice(&self) -> &Invoice {
        &self.invoice
    }

    /// Returns the billing adapter that reported final success.
    pub fn provider(&self) -> &'static str {
        self.provider
    }

    /// Returns the opaque provider charge identifier.
    pub fn charge_id(&self) -> &str {
        &self.charge_id
    }

    /// Stable, non-secret key for an application-owned delivery outbox.
    pub fn delivery_key(&self) -> &str {
        &self.delivery_key
    }
}

impl std::fmt::Debug for PaidInvoice {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PaidInvoice")
            .field("invoice_id", &self.invoice.invoice_id)
            .field("customer_email", &"[REDACTED]")
            .field("provider", &self.provider)
            .field("charge_id", &"[REDACTED]")
            .field("delivery_key", &self.delivery_key)
            .finish()
    }
}

impl Invoice {
    /// Binds this invoice to a final successful direct-charge receipt.
    pub fn bind_succeeded_charge(
        &self,
        receipt: &ChargeReceipt,
    ) -> Result<PaidInvoice, crate::error::CapitalError> {
        let money = self.validated_money()?;
        if receipt.status() != ChargeStatus::Succeeded {
            return Err(invalid_invoice(
                "invoice delivery requires a final succeeded charge receipt",
            ));
        }
        if receipt.amount_minor() != money.total_minor
            || !receipt.currency().eq_ignore_ascii_case(&self.currency)
        {
            return Err(invalid_invoice(
                "invoice amount and currency must match the succeeded charge receipt",
            ));
        }
        if receipt.customer_email() != self.customer_email {
            return Err(invalid_invoice(
                "invoice recipient must exactly match the succeeded charge receipt",
            ));
        }

        let delivery_key = delivery_key(self, receipt);
        Ok(PaidInvoice {
            invoice: self.clone(),
            provider: receipt.provider(),
            charge_id: receipt.charge_id().to_string(),
            delivery_key,
        })
    }
}

fn delivery_key(invoice: &Invoice, receipt: &ChargeReceipt) -> String {
    let mut material = Vec::with_capacity(512);
    for part in [
        "rullst-capital:paid-invoice-delivery:v1",
        invoice.invoice_id.as_str(),
        receipt.provider(),
        receipt.charge_id(),
        receipt.currency(),
    ] {
        material.extend_from_slice(part.as_bytes());
        material.push(0);
    }
    material.extend_from_slice(&receipt.amount_minor().to_be_bytes());
    format!("paid_invoice_{}", hex::encode(digest(&SHA256, &material)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChargeStatus, InvoiceItem};
    use chrono::Utc;

    fn invoice() -> Invoice {
        Invoice {
            invoice_id: "INV-42".to_string(),
            customer_email: "owner@example.com".to_string(),
            date: Utc::now(),
            items: vec![InvoiceItem {
                description: "Subscription".to_string(),
                amount: 49.90,
            }],
            total: 49.90,
            currency: "BRL".to_string(),
        }
    }

    fn receipt(status: ChargeStatus) -> ChargeReceipt {
        ChargeReceipt::from_verified_provider_response(
            "fixture",
            "charge_42",
            status,
            4_990,
            "brl",
            "owner@example.com",
        )
        .expect("valid fixture")
    }

    #[test]
    fn binding_requires_final_exact_payment_evidence_and_is_stable() {
        let invoice = invoice();
        let first = invoice
            .bind_succeeded_charge(&receipt(ChargeStatus::Succeeded))
            .expect("paid invoice");
        let second = invoice
            .bind_succeeded_charge(&receipt(ChargeStatus::Succeeded))
            .expect("stable paid invoice");
        assert_eq!(first.delivery_key(), second.delivery_key());
        assert_eq!(first.invoice().invoice_id, "INV-42");
        assert!(!format!("{first:?}").contains("owner@example.com"));
        assert!(!format!("{first:?}").contains("charge_42"));

        assert!(
            invoice
                .bind_succeeded_charge(&receipt(ChargeStatus::Processing))
                .is_err()
        );
        assert!(
            invoice
                .bind_succeeded_charge(&receipt(ChargeStatus::Mock))
                .is_err()
        );

        for invalid in [
            ChargeReceipt::from_verified_provider_response(
                "fixture",
                "charge_42",
                ChargeStatus::Succeeded,
                4_991,
                "brl",
                "owner@example.com",
            )
            .expect("structural fixture"),
            ChargeReceipt::from_verified_provider_response(
                "fixture",
                "charge_42",
                ChargeStatus::Succeeded,
                4_990,
                "usd",
                "owner@example.com",
            )
            .expect("structural fixture"),
            ChargeReceipt::from_verified_provider_response(
                "fixture",
                "charge_42",
                ChargeStatus::Succeeded,
                4_990,
                "brl",
                "other@example.com",
            )
            .expect("structural fixture"),
        ] {
            assert!(invoice.bind_succeeded_charge(&invalid).is_err());
        }
    }
}
