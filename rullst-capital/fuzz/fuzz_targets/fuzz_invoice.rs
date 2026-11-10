#![no_main]

use chrono::Utc;
use libfuzzer_sys::fuzz_target;
use rullst_capital::invoice::{Invoice, InvoiceLocale};

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let mut invoice = Invoice::new("INV-100", s, Utc::now(), "USD")
            .with_item(s, 9_900)
            .with_locale(InvoiceLocale::from_tag(s).unwrap_or_default());
        invoice.labels.title = s.to_string();
        let _ = invoice.generate_html();
        let _ = invoice.try_generate_html();
    }
});
