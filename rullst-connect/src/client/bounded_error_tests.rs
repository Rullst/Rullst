//! Provider error text is bounded on UTF-8 character boundaries.

use super::request_builder::ResponseWrapper;
use super::traits::HttpResponse;
use crate::error::ConnectError;
use serde_json::{Value, json};

const MARKER: &str = "... (truncated)";

fn provider_api_error(status: u16, body: Value) -> (String, String) {
    let wrapper = ResponseWrapper {
        res: HttpResponse { status, body },
    };
    match wrapper.error_for_status() {
        Err(ConnectError::ProviderApiError { code, message }) => (code, message),
        other => panic!("expected ProviderApiError, got {other:?}"),
    }
}

/// 511 ASCII bytes followed by a two-byte character that spans bytes 511..513.
fn straddling_text() -> String {
    format!("{}ção inválida", "a".repeat(511))
}

#[test]
fn multibyte_character_across_the_message_limit_does_not_panic() {
    let text = straddling_text();
    assert!(
        !text.is_char_boundary(512),
        "fixture must straddle byte 512"
    );

    let bodies = [
        json!({ "error": "invalid_grant", "error_description": text }),
        json!({ "message": text }),
        Value::String(text.clone()),
    ];
    for body in bodies {
        let (_, message) = provider_api_error(400, body);
        assert!(message.ends_with(MARKER), "{message}");
        let kept = message.strip_suffix(MARKER).expect("truncation marker");
        assert_eq!(kept, "a".repeat(511), "cut at the last boundary <= 512");
    }
}

#[test]
fn whole_json_body_fallback_is_bounded_on_a_character_boundary() {
    // The serialized body is `{"detail":"<500 x a>é…"}`, placing `é` at bytes 511..513.
    let body = json!({ "detail": format!("{}é{}", "a".repeat(500), "b".repeat(100)) });
    let serialized = body.to_string();
    assert!(
        !serialized.is_char_boundary(512),
        "fixture must straddle byte 512"
    );

    let (_, message) = provider_api_error(502, body);
    let kept = message.strip_suffix(MARKER).expect("truncation marker");
    assert_eq!(kept.len(), 511);
    assert!(serialized.starts_with(kept));
}

#[test]
fn provider_error_code_is_bounded_on_a_character_boundary() {
    let code = format!("{}ã{}", "e".repeat(127), "x".repeat(4096));
    let (bounded_code, message) =
        provider_api_error(401, json!({ "error": code, "error_description": "denied" }));
    assert_eq!(message, "denied");
    let kept = bounded_code
        .strip_suffix(MARKER)
        .expect("truncation marker");
    assert_eq!(kept, "e".repeat(127));
}

#[test]
fn short_localized_errors_are_preserved_exactly() {
    let (code, message) = provider_api_error(
        400,
        json!({ "error": "requisição_inválida", "error_description": "Código expirado" }),
    );
    assert_eq!(code, "requisição_inválida");
    assert_eq!(message, "Código expirado");

    let exact = format!("{}é", "a".repeat(510));
    assert_eq!(exact.len(), 512);
    let (_, message) = provider_api_error(400, Value::String(exact.clone()));
    assert_eq!(message, exact, "exactly 512 bytes is not truncated");
}

#[test]
fn successful_json_error_responses_are_bounded() {
    let description = format!("{}ç", "d".repeat(2048));
    let error = crate::error::provider_returned_error("invalid_grant", &description);
    let ConnectError::ProviderApiError { code, message } = error else {
        panic!("expected a structured provider error");
    };
    assert_eq!(code, "invalid_grant");
    assert!(message.starts_with("ddd"));
    assert!(message.len() < 600, "{} bytes", message.len());

    let code = format!("{}ç", "c".repeat(127));
    let ConnectError::ProviderApiError { code, message } =
        crate::error::provider_returned_error(&code, "")
    else {
        panic!("expected a structured provider error");
    };
    assert_eq!(code, format!("{}{MARKER}", "c".repeat(127)));
    assert_eq!(message, "Unknown error");
}
