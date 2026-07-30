#![no_main]

use libfuzzer_sys::fuzz_target;
use rullst_connect::user::ConnectUser;

fuzz_target!(|data: &[u8]| {
    // Target: Test the robustness of the JSON parser for OAuth user profiles.
    // Different providers may return unexpected data types or missing fields.
    // This ensures the library returns a friendly error (Err)
    // instead of crashing the server with a panic.

    
    // LibFuzzer/ASan runs out of memory (RSS limit) when fuzzing highly nested JSON 
    // into serde_json::Value due to the overhead of tracking thousands of small allocations.
    // We restrict the depth here to prevent the fuzzer from timing out or hitting OOM.
    let mut depth: usize = 0;
    for &b in data {
        if b == b'{' || b == b'[' {
            depth += 1;
            if depth > 32 {
                return;
            }
        } else if b == b'}' || b == b']' {
            depth = depth.saturating_sub(1);
        }
    }

    if let Ok(s) = std::str::from_utf8(data) {
        let _ = serde_json::from_str::<ConnectUser>(s);
    }
});
