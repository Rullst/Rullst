#![no_main]

use libfuzzer_sys::fuzz_target;
use rullst_studio::data_browser::db::sanitize_identifier;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let clean = sanitize_identifier(s);
        assert!(clean.len() <= 64);
    }
});
