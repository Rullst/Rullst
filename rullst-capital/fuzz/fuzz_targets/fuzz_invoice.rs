#![no_main]

use chrono::Utc;
use libfuzzer_sys::fuzz_target;
use rullst_capital::invoice::{Invoice, InvoiceItem};

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let invoice = Invoice {
            invoice_id: "INV-100".to_string(),
            customer_email: s.to_string(),
            date: Utc::now(),
            items: vec![InvoiceItem {
                description: s.to_string(),
                amount: 99.0,
            }],
            total: 99.0,
            currency: "USD".to_string(),
        };
        let _ = invoice.generate_html();
    }
});
