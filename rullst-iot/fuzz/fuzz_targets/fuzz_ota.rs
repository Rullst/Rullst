#![no_main]

use libfuzzer_sys::fuzz_target;
use rullst_iot::ota::OtaManager;

fuzz_target!(|data: &[u8]| {
    if data.len() >= 64 {
        let payload = &data[64..];
        let signature = &data[..64];
        let mut manager = OtaManager::new("1.0.0");
        let _ = manager.verify_signature(payload, signature);
        if let Ok(v) = std::str::from_utf8(payload) {
            let _ = manager.commit_update(v);
        }
    }
});
