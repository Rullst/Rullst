#![no_main]

use libfuzzer_sys::fuzz_target;
use rullst_security::vault::FieldEncryptor;

fuzz_target!(|data: &[u8]| {
    const KEY: &[u8; 32] = b"0123456789abcdef0123456789abcdef";

    if let Ok(s) = std::str::from_utf8(data) {
        let _ = FieldEncryptor::decrypt_with_aad(s, KEY, b"fuzz:field");
    }
});
