//! Bounds of the revocation token accepted before any revocation request.
use super::validate_revocation_token;
use crate::ConnectError;

#[test]
fn revocation_tokens_hold_one_to_16_kib_bytes() {
    assert!(validate_revocation_token(&"r".repeat(16_384)).is_ok());
    for token in [String::new(), "r".repeat(16_385)] {
        assert!(
            matches!(
                validate_revocation_token(&token),
                Err(ConnectError::Token(_))
            ),
            "accepted a token of {} bytes",
            token.len()
        );
    }
}
