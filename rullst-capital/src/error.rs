/// Strongly-typed error domain for Rullst Capital and SaaS billing operations.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CapitalError {
    /// Authentication with payment provider failed.
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    /// Missing or invalid configuration (API key, webhook secret, etc.).
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// Webhook signature verification failed.
    #[error("Invalid webhook signature: {0}")]
    InvalidSignature(String),

    /// A signed webhook is outside the provider's accepted timestamp window.
    #[error("Stale webhook: {0}")]
    StaleWebhook(String),

    /// A webhook payload was already accepted inside the replay-protection window.
    #[error("Webhook replay detected: {0}")]
    WebhookReplay(String),

    /// A mock verifier was mounted on the production-safe webhook middleware.
    #[error("Mock webhook mode is not allowed by this middleware: {0}")]
    MockWebhookNotAllowed(String),

    /// Webhook payload parsing failed.
    #[error("Failed to parse webhook payload: {0}")]
    PayloadParseError(String),

    /// Provider API request failed.
    #[error("Payment provider API request failed: {0}")]
    ProviderRequestFailed(String),

    /// The requested operation is not supported by the payment provider.
    #[error("Operation not supported by provider: {0}")]
    UnsupportedOperation(String),

    /// An error occurred during subscription lifecycle management.
    #[error("Subscription error: {0}")]
    SubscriptionError(String),

    /// A direct-charge request violates a local validation invariant.
    #[error("Invalid charge request: {0}")]
    InvalidCharge(String),

    /// Digital invoice or tax authority operation error.
    #[error("Fiscal error: {0}")]
    FiscalError(#[from] crate::fiscal::models::FiscalError),

    /// General billing error.
    #[error("Billing error: {0}")]
    General(String),
}

impl From<String> for CapitalError {
    fn from(err: String) -> Self {
        CapitalError::General(err)
    }
}

impl From<&str> for CapitalError {
    fn from(err: &str) -> Self {
        CapitalError::General(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fiscal::models::FiscalError;

    #[test]
    fn test_capital_error_display_and_conversions() {
        let e1 = CapitalError::AuthenticationFailed("bad token".to_string());
        assert_eq!(e1.to_string(), "Authentication failed: bad token");

        let e2 = CapitalError::ConfigurationError("missing key".to_string());
        assert_eq!(e2.to_string(), "Configuration error: missing key");

        let e3 = CapitalError::InvalidSignature("sig mismatch".to_string());
        assert_eq!(e3.to_string(), "Invalid webhook signature: sig mismatch");

        let e4 = CapitalError::PayloadParseError("bad json".to_string());
        assert_eq!(e4.to_string(), "Failed to parse webhook payload: bad json");

        let e5 = CapitalError::ProviderRequestFailed("500 internal".to_string());
        assert_eq!(
            e5.to_string(),
            "Payment provider API request failed: 500 internal"
        );

        let e6 = CapitalError::UnsupportedOperation("pause not supported".to_string());
        assert_eq!(
            e6.to_string(),
            "Operation not supported by provider: pause not supported"
        );

        let e7 = CapitalError::SubscriptionError("sub not found".to_string());
        assert_eq!(e7.to_string(), "Subscription error: sub not found");

        let f_err = FiscalError::XmlSigning("bad xml".to_string());
        let e8: CapitalError = f_err.into();
        assert!(
            e8.to_string()
                .contains("Fiscal error: XML digital signing error")
        );

        let e9: CapitalError = "from str".into();
        assert_eq!(e9, CapitalError::General("from str".to_string()));

        let e10: CapitalError = "from string".to_string().into();
        assert_eq!(e10, CapitalError::General("from string".to_string()));
    }
}
