#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() <= 2048 {
        if let Ok(text) = std::str::from_utf8(data) {
            let _ = rullst_fuzz::tenants(text);
        }
    }
});
