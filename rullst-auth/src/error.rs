/// Strongly-typed error domain for Rullst Authentication & Session Management.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
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
