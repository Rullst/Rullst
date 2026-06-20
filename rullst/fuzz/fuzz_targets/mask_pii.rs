#![no_main]

use libfuzzer_sys::fuzz_target;
use rullst::security::mask_pii;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = mask_pii(s);
    }
});
