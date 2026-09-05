#![no_main]

use libfuzzer_sys::fuzz_target;
use rullst_security::dlp::mask_response_payload;

fuzz_target!(|data: &[u8]| {
    let _ = mask_response_payload(data);
});
