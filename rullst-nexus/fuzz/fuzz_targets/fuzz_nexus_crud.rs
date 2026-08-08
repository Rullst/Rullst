#![no_main]

use libfuzzer_sys::fuzz_target;
use rullst_nexus::nexus::crud::sanitize_identifier;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let clean = sanitize_identifier(s);
        for ch in clean.chars() {
            assert!(ch.is_alphanumeric() || ch == '_');
        }
    }
});
