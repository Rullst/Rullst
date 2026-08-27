#![no_main]

use libfuzzer_sys::fuzz_target;
use rullst_ai::ai::tools::{ToolExecutionPolicy, ToolParam, ToolRegistry};

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = ToolParam {
            name: s.to_string(),
            param_type: "string".to_string(),
            description: s.to_string(),
            required: data.len() % 2 == 0,
        };
        let registry = ToolRegistry::new();
        if let Ok(policy) = ToolExecutionPolicy::new([s]) {
            let _ = registry.export_openai_schema(&policy);
        }
    }
});
