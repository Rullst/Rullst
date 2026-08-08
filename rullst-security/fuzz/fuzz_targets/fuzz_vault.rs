#![no_main]

use libfuzzer_sys::fuzz_target;
use rullst_security::vault::FieldEncryptor;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = FieldEncryptor::decrypt(s, "test_secret_key");
    }
});
