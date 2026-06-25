#![no_main]

use libfuzzer_sys::fuzz_target;
use rullst::html::escape_str;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = escape_str(s);
    }
});
