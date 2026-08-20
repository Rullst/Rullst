#![no_main]

use libfuzzer_sys::fuzz_target;
use rullst_ai::ai::Message;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let msg = Message {
            role: "user".to_string(),
            content: s.to_string(),
        };
        if let Ok(serialized) = serde_json::to_string(&msg) {
            let _: Result<Message, _> = serde_json::from_str(&serialized);
        }
        let _: Result<Message, _> = serde_json::from_str(s);
    }
});
