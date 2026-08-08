#![no_main]

use libfuzzer_sys::fuzz_target;
use rullst_ai::ai::rag::build_rag_prompt;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let contexts = vec![s.to_string(), "context fallback".to_string()];
        let prompt = build_rag_prompt(s, &contexts);
        assert!(prompt.contains("Question:"));
        assert!(prompt.contains("Answer:"));
    }
});
