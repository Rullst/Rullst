#![no_main]
use libfuzzer_sys::fuzz_target;
use rullst::security::generate_csrf_token;

fuzz_target!(|data: &[u8]| {
    let _ = data;
    let token = generate_csrf_token();
    assert_eq!(token.len(), 32);
});
