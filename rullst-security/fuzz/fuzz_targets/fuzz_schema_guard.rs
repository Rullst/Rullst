#![no_main]

use libfuzzer_sys::fuzz_target;
use rullst_security::schema_guard::inspect_json_payload;

fuzz_target!(|data: &[u8]| {
    let _ = inspect_json_payload(data, 32, 2 * 1024 * 1024);
});
