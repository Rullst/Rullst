/// Strongly-typed error domain for Rullst Capital and SaaS billing operations.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
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
