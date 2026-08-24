#![no_main]

use libfuzzer_sys::fuzz_target;
use rullst_iot::ota::{OtaManager, OtaManifest};

// RFC 8032 test-vector public key. The corresponding private key is not used by
// this target; arbitrary signatures must be rejected without panicking.
const TRUSTED_PUBLIC_KEY: [u8; 32] = [
    0xd7, 0x5a, 0x98, 0x01, 0x82, 0xb1, 0x0a, 0xb7, 0xd5, 0x4b, 0xfe, 0xd3, 0xc9, 0x64, 0x07, 0x3a,
    0x0e, 0xe1, 0x72, 0xf3, 0xda, 0xa6, 0x23, 0x25, 0xaf, 0x02, 0x1a, 0x68, 0xf7, 0x07, 0x51, 0x1a,
];

fuzz_target!(|data: &[u8]| {
    if let Some((&signature_selector, remaining)) = data.split_first() {
        let signature_len = usize::from(signature_selector) % 65;
        let split_at = core::cmp::min(signature_len, remaining.len());
        let (signature, firmware) = remaining.split_at(split_at);
        if let (Ok(mut manager), Ok(manifest)) = (
            OtaManager::new_with_trusted_key("fuzz-board", "1.0.0", 1, TRUSTED_PUBLIC_KEY),
            OtaManifest::from_firmware("fuzz-board", "2.0.0", 2, firmware),
        ) {
            let _ = manager.verify_update(&manifest, firmware, signature);
            let _ = manager.commit_verified_update();
        }
    }
});
