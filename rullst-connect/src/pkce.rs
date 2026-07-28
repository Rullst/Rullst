use base64::{Engine as _, engine::general_purpose};
use rand::{RngExt, distr::Alphanumeric};
use sha2::{Digest, Sha256};

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
    fn test_generate_pkce_uniqueness() {
        let (verifier1, challenge1) = generate_pkce();
        let (verifier2, challenge2) = generate_pkce();

        assert_ne!(
            verifier1, verifier2,
            "Multiple calls should generate unique verifiers"
        );
        assert_ne!(
            challenge1, challenge2,
            "Multiple calls should generate unique challenges"
        );
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
