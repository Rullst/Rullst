#![no_main]
use libfuzzer_sys::fuzz_target;
use rullst::auth::decrypt_session;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let dummy_key = [42u8; 32];
        let _ = decrypt_session(s, &dummy_key);
    }
});
