#![no_main]

use libfuzzer_sys::fuzz_target;
use rullst_mail::validator::{
    is_disposable_domain, is_disposable_email, validate_email_deliverability, validate_email_syntax,
};

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = validate_email_syntax(s);
        let _ = validate_email_deliverability(s);
        let _ = is_disposable_email(s);
        let _ = is_disposable_domain(s);
    }
});
