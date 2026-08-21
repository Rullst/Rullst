/// Strongly-typed error domain for Rullst Security, RASP, Vault, Audit & RBAC subsystems.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum SecurityError {
    /// Action is unauthorized (missing or invalid credentials/identity).
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// Action is forbidden (authenticated entity lacks necessary role or ownership).
    #[error("Forbidden: {0}")]
    Forbidden(String),

    /// Cryptographic vault / encryption / decryption failure.
    #[error("Vault error: {0}")]
    VaultError(String),

    /// Tamper-evident audit chain error or integrity validation failure.
    #[error("Audit chain error: {0}")]
    AuditChainError(String),

    /// Input sanitization or injection prevention failure.
    #[error("Sanitization error: {0}")]
    SanitizationError(String),

    /// Dynamic SQL identifier or schema validation failure.
    #[error("Schema guard violation: {0}")]
    SchemaGuardError(String),

    /// Request blocked by RASP / WAF rule.
    #[error("Security WAF blocked request: {0}")]
    WafBlocked(String),

    /// General security exception.
    #[error("Security error: {0}")]
    General(String),
}

impl From<String> for SecurityError {
    fn from(err: String) -> Self {
        SecurityError::General(err)
    }
}

impl From<&str> for SecurityError {
    fn from(err: &str) -> Self {
        SecurityError::General(err.to_string())
    }
}
