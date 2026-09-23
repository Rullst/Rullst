#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() <= 2048 {
        if let Ok(text) = std::str::from_utf8(data) {
            assert_eq!(rullst_fuzz::realtime(text).expect("bounded payload"), text);
        }
    }
});
