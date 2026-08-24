/// Strongly-typed error domain for Rullst Authentication & Session Management.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AuthError {
    /// Invalid login credentials (incorrect username, email, or password).
    #[error("Invalid credentials")]
    InvalidCredentials,

    /// Password hashing failed (e.g. Argon2 error).
    #[error("Password hashing failed: {0}")]
    PasswordHashError(String),

    /// Password verification failed.
    #[error("Password verification failed: {0}")]
    PasswordVerifyError(String),

    /// Session token encryption failed.
    #[error("Session encryption failed: {0}")]
    SessionEncryptionError(String),

    /// Session token decryption or deserialization failed.
    #[error("Session decryption failed: {0}")]
    SessionDecryptionError(String),

    /// The session token has expired.
    #[error("Session token expired")]
    SessionExpired,

    /// The application key `APP_KEY` is missing or has invalid format.
    #[error("APP_KEY configuration error: {0}")]
    MissingAppKey(String),

    /// WebAuthn / Passkey registration or assertion error.
    #[error("Passkey error: {0}")]
    PasskeyError(String),

    /// CBOR decoding error during WebAuthn attestation/assertion parsing.
    #[error("CBOR parse error: {0}")]
    CborParseError(String),

    /// Access is forbidden / unauthorized.
    #[error("Unauthorized access: {0}")]
    Unauthorized(String),

    /// General authentication error.
    #[error("Authentication error: {0}")]
    General(String),
}

impl From<String> for AuthError {
    fn from(err: String) -> Self {
        AuthError::General(err)
    }
}

impl From<&str> for AuthError {
    fn from(err: &str) -> Self {
        AuthError::General(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_error_display_and_conversions() {
        assert_eq!(
            AuthError::InvalidCredentials.to_string(),
            "Invalid credentials"
        );
        assert_eq!(
            AuthError::PasswordHashError("argon2".to_string()).to_string(),
            "Password hashing failed: argon2"
        );
        assert_eq!(
            AuthError::PasswordVerifyError("mismatch".to_string()).to_string(),
            "Password verification failed: mismatch"
        );
        assert_eq!(
            AuthError::SessionEncryptionError("key".to_string()).to_string(),
            "Session encryption failed: key"
        );
        assert_eq!(
            AuthError::SessionDecryptionError("corrupt".to_string()).to_string(),
            "Session decryption failed: corrupt"
        );
        assert_eq!(
            AuthError::SessionExpired.to_string(),
            "Session token expired"
        );
        assert_eq!(
            AuthError::MissingAppKey("not set".to_string()).to_string(),
            "APP_KEY configuration error: not set"
        );
        assert_eq!(
            AuthError::PasskeyError("invalid origin".to_string()).to_string(),
            "Passkey error: invalid origin"
        );
        assert_eq!(
            AuthError::CborParseError("eof".to_string()).to_string(),
            "CBOR parse error: eof"
        );
        assert_eq!(
            AuthError::Unauthorized("admin only".to_string()).to_string(),
            "Unauthorized access: admin only"
        );

        let e1: AuthError = "general error".into();
        assert_eq!(e1, AuthError::General("general error".to_string()));

        let e2: AuthError = "string err".to_string().into();
        assert_eq!(e2, AuthError::General("string err".to_string()));
    }
}
