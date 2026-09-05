#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Attempt to convert the random bytes into a UTF-8 string
    if let Ok(s) = std::str::from_utf8(data) {
        // Split the string in half safely (respecting UTF-8 character boundaries)
        let mid = s.floor_char_boundary(s.len() / 2);
        let (old_json, new_json) = s.split_at(mid);

        // Fuzzing the audit diff function
        // The goal is to ensure this function never causes a 'panic!'
        // even with invalid JSONs, halved or containing obscure characters.
        let _ = rullst_orm::audit::compute_diff(old_json, new_json);
    }
});
