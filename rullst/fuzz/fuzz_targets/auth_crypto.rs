#![no_main]
use libfuzzer_sys::fuzz_target;
use rullst::auth::{hash_password, verify_password, needs_rehash};

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        if s.len() <= 72 {
            if let Ok(hash) = hash_password(s) {
                let _ = verify_password(s, &hash);
                let _ = needs_rehash(&hash);
            }
        }
        
        // We only test needs_rehash for random strings, because verify_password
        // will attempt to allocate memory based on the `m=` parameter parsed from the
        // arbitrary fuzzer string, leading to trivial OOMs. In reality, the hash comes
        // from the database, not user input.
        let _ = needs_rehash(s);
    }
});
