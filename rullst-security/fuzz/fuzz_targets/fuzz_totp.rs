#![no_main]

use libfuzzer_sys::fuzz_target;
use rullst_security::mfa::verify_totp_code;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = verify_totp_code("JBSWY3DPEHPK3PXP", s);
    }
});
