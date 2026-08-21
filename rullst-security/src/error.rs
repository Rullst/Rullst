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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_error_display_and_conversions() {
        assert_eq!(
            SecurityError::Unauthorized("no token".to_string()).to_string(),
            "Unauthorized: no token"
        );
        assert_eq!(
            SecurityError::Forbidden("no admin".to_string()).to_string(),
            "Forbidden: no admin"
        );
        assert_eq!(
            SecurityError::VaultError("decrypt".to_string()).to_string(),
            "Vault error: decrypt"
        );
        assert_eq!(
            SecurityError::AuditChainError("tamper".to_string()).to_string(),
            "Audit chain error: tamper"
        );
        assert_eq!(
            SecurityError::SanitizationError("xss".to_string()).to_string(),
            "Sanitization error: xss"
        );
        assert_eq!(
            SecurityError::SchemaGuardError("invalid col".to_string()).to_string(),
            "Schema guard violation: invalid col"
        );
        assert_eq!(
            SecurityError::WafBlocked("sqli".to_string()).to_string(),
            "Security WAF blocked request: sqli"
        );

        let e1: SecurityError = "sec error".into();
        assert_eq!(e1, SecurityError::General("sec error".to_string()));

        let e2: SecurityError = "str err".to_string().into();
        assert_eq!(e2, SecurityError::General("str err".to_string()));
    }
}
