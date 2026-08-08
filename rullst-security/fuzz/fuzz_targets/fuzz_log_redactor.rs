#![no_main]

use libfuzzer_sys::fuzz_target;
use rullst_security::log_redactor::redact_secrets;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = redact_secrets(s);
    }
});
