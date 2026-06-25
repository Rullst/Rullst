#![no_main]
use libfuzzer_sys::fuzz_target;
use rullst::validation::ValidationError;
use axum::response::IntoResponse;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let err_htmx = ValidationError::ExtractionError {
            message: s.to_string(),
            is_htmx: true,
        };
        let _ = err_htmx.into_response();

        let err_json = ValidationError::ExtractionError {
            message: s.to_string(),
            is_htmx: false,
        };
        let _ = err_json.into_response();
    }
});
