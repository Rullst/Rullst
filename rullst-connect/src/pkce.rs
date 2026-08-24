use base64::{Engine as _, engine::general_purpose};
use rand::{RngExt, distr::Alphanumeric};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// Generates a (code_verifier, code_challenge) pair for OAuth2 PKCE.
///
/// - `code_verifier`: A high-entropy cryptographic random string. The developer MUST store this in the session/cookie.
/// - `code_challenge`: The base64-url-encoded SHA256 hash of the verifier. Sent in the authorization URL.
pub fn generate_pkce() -> (String, String) {
    // Generate a 64-character random string (verifier)
    let mut code_verifier = String::with_capacity(64);
    code_verifier.extend(
        rand::rng()
            .sample_iter(&Alphanumeric)
            .take(64)
            .map(char::from),
    );

    // SHA256 hash
    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    let result = hasher.finalize();

    // Base64-url encoding without padding
    let code_challenge = general_purpose::URL_SAFE_NO_PAD.encode(result);

    (code_verifier, code_challenge)
}

/// Verifies that a given `code_verifier` matches the expected `code_challenge` using constant-time comparison to prevent timing attacks.
pub fn verify_pkce_challenge(code_verifier: &str, expected_challenge: &str) -> bool {
    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    let result = hasher.finalize();
    let computed_challenge = general_purpose::URL_SAFE_NO_PAD.encode(result);

    computed_challenge
        .as_bytes()
        .ct_eq(expected_challenge.as_bytes())
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_pkce_length() {
        let (verifier, _) = generate_pkce();
        assert_eq!(
            verifier.len(),
            64,
            "Code verifier should be 64 characters long"
        );
    }

    #[test]
    fn test_generate_pkce_challenge_format() {
        let (verifier, challenge) = generate_pkce();

        // Compute expected challenge
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let result = hasher.finalize();
        let expected_challenge = general_purpose::URL_SAFE_NO_PAD.encode(result);

        assert_eq!(
            challenge, expected_challenge,
            "Challenge should match base64-url-encoded SHA256 of verifier"
        );
        assert!(
            !challenge.contains('='),
            "Challenge should not contain padding characters"
        );
    }

    #[test]
    fn test_verify_pkce_challenge() {
        let (verifier, challenge) = generate_pkce();
        assert!(verify_pkce_challenge(&verifier, &challenge));
        assert!(!verify_pkce_challenge(&verifier, "invalid_challenge_hash"));
        assert!(!verify_pkce_challenge(
            "wrong_verifier_string_123456",
            &challenge
        ));
    }

    use proptest::prelude::*;
    proptest! {
        #[test]
        fn test_pkce_challenge_properties(verifier in "[a-zA-Z0-9-._~]{43,128}") {
            // Compute expected challenge
            let mut hasher = Sha256::new();
            hasher.update(verifier.as_bytes());
            let result = hasher.finalize();
            let expected_challenge = general_purpose::URL_SAFE_NO_PAD.encode(result);

            // Assert it does not have padding
            prop_assert!(!expected_challenge.contains('='));
            // Assert it's url safe (no + or /)
            prop_assert!(!expected_challenge.contains('+'));
            prop_assert!(!expected_challenge.contains('/'));
        }
    }
}
