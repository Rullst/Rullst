#![no_main]

use libfuzzer_sys::fuzz_target;
use rullst_security::sanitizer::sanitize_identifier;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = sanitize_identifier(s);
    }
});
