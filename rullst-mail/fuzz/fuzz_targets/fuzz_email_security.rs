#![no_main]

use libfuzzer_sys::fuzz_target;
use rullst_mail::Message;
use rullst_mail::security::{extract_urls, is_dangerous_scheme, is_homograph_domain, scan_content_security};

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = is_homograph_domain(s);
        let _ = is_dangerous_scheme(s);
        let _ = extract_urls(s);
        let _ = scan_content_security(s);

        let msg = Message::new().to(s).subject(s).html(s).text(s);
        let _ = msg.validate_deliverability();
        let _ = msg.is_disposable();
    }
});
