use std::time::Duration;

/// Stable retry disposition for an outbound provider failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProviderFailureClass {
    /// Repeating the same request is not expected to repair the failure.
    Permanent,
    /// A later retry may succeed when the operation is independently idempotent.
    Transient,
    /// The provider asked the caller to reduce request frequency.
    RateLimited,
}

/// Low-cardinality reason for an outbound provider failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProviderFailureKind {
    /// The outbound request could not be constructed.
    RequestBuild,
    /// The request or response transport failed.
    Transport,
    /// The provider returned a non-success HTTP status.
    HttpResponse,
    /// The successful response exceeded the documented byte limit.
    ResponseTooLarge,
    /// The successful response did not contain valid JSON.
    InvalidResponse,
    /// JSON decoded, but required provider evidence was absent or inconsistent.
    ContractMismatch,
}

/// Redacted, deterministic failure evidence shared by live provider adapters.
///
/// This value intentionally excludes request URLs, credentials, raw response bodies, and
/// transport diagnostics. A retry classification is not permission to repeat a payment:
/// callers still need a provider-forwarded idempotency key and reconciliation policy.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ProviderFailure {
    provider: &'static str,
    operation: &'static str,
    kind: ProviderFailureKind,
    status: Option<u16>,
    retry_after: Option<Duration>,
}

impl ProviderFailure {
    pub(crate) fn request_build(provider: &'static str, operation: &'static str) -> Self {
        Self::new(
            provider,
            operation,
            ProviderFailureKind::RequestBuild,
            None,
            None,
        )
    }

    pub(crate) fn transport(provider: &'static str, operation: &'static str) -> Self {
        Self::new(
            provider,
            operation,
            ProviderFailureKind::Transport,
            None,
            None,
        )
    }

    pub(crate) fn http_response(
        provider: &'static str,
        operation: &'static str,
        status: u16,
        retry_after: Option<Duration>,
    ) -> Self {
        Self::new(
            provider,
            operation,
            ProviderFailureKind::HttpResponse,
            Some(status),
            retry_after,
        )
    }

    pub(crate) fn response_too_large(provider: &'static str, operation: &'static str) -> Self {
        Self::new(
            provider,
            operation,
            ProviderFailureKind::ResponseTooLarge,
            None,
            None,
        )
    }

    pub(crate) fn invalid_response(provider: &'static str, operation: &'static str) -> Self {
        Self::new(
            provider,
            operation,
            ProviderFailureKind::InvalidResponse,
            None,
            None,
        )
    }

    pub(crate) fn contract_mismatch(provider: &'static str, operation: &'static str) -> Self {
        Self::new(
            provider,
            operation,
            ProviderFailureKind::ContractMismatch,
            None,
            None,
        )
    }

    fn new(
        provider: &'static str,
        operation: &'static str,
        kind: ProviderFailureKind,
        status: Option<u16>,
        retry_after: Option<Duration>,
    ) -> Self {
        Self {
            provider,
            operation,
            kind,
            status,
            retry_after,
        }
    }

    /// Adapter label without account, customer, or credential material.
    pub fn provider(&self) -> &'static str {
        self.provider
    }

    /// Static operation label suitable for low-cardinality telemetry.
    pub fn operation(&self) -> &'static str {
        self.operation
    }

    /// Structural reason for the failure.
    pub fn kind(&self) -> ProviderFailureKind {
        self.kind
    }

    /// Provider HTTP status when a response was received.
    pub fn status(&self) -> Option<u16> {
        self.status
    }

    /// Bounded numeric `Retry-After` delay supplied by the provider, when present.
    pub fn retry_after(&self) -> Option<Duration> {
        self.retry_after
    }

    /// Deterministic failure class for caller-owned retry and alerting policy.
    pub fn class(&self) -> ProviderFailureClass {
        match (self.kind, self.status) {
            (ProviderFailureKind::Transport, _) => ProviderFailureClass::Transient,
            (ProviderFailureKind::HttpResponse, Some(429)) => ProviderFailureClass::RateLimited,
            (ProviderFailureKind::HttpResponse, Some(408 | 425 | 500..=599)) => {
                ProviderFailureClass::Transient
            }
            _ => ProviderFailureClass::Permanent,
        }
    }

    /// Whether a later retry may be considered after idempotency has been established.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self.class(),
            ProviderFailureClass::Transient | ProviderFailureClass::RateLimited
        )
    }
}

impl std::fmt::Display for ProviderFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.kind, self.status) {
            (ProviderFailureKind::RequestBuild, _) => write!(
                formatter,
                "{} {} request construction failed",
                self.provider, self.operation
            ),
            (ProviderFailureKind::Transport, _) => {
                write!(
                    formatter,
                    "{} {} transport failed",
                    self.provider, self.operation
                )
            }
            (ProviderFailureKind::HttpResponse, Some(status)) => write!(
                formatter,
                "{} {} returned HTTP {}",
                self.provider, self.operation, status
            ),
            (ProviderFailureKind::HttpResponse, None) => write!(
                formatter,
                "{} {} returned an unsuccessful response",
                self.provider, self.operation
            ),
            (ProviderFailureKind::ResponseTooLarge, _) => write!(
                formatter,
                "{} {} response exceeded 1 MiB",
                self.provider, self.operation
            ),
            (ProviderFailureKind::InvalidResponse, _) => write!(
                formatter,
                "{} {} returned malformed JSON",
                self.provider, self.operation
            ),
            (ProviderFailureKind::ContractMismatch, _) => write!(
                formatter,
                "{} {} response failed contract validation",
                self.provider, self.operation
            ),
        }
    }
}

impl std::error::Error for ProviderFailure {}

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

    /// The bounded replay ledger reached its configured active-record limit.
    #[error("Webhook replay store is full")]
    WebhookReplayStoreFull,

    /// The durable replay ledger could not complete a required operation.
    #[error("Webhook replay store is unavailable")]
    WebhookReplayStoreUnavailable,

    /// Existing durable replay metadata does not match the configured profile.
    #[error("Webhook replay store configuration differs from persisted metadata")]
    WebhookReplayConfigurationDrift,

    /// Persisted replay state violates the fixed schema or value contract.
    #[error("Webhook replay store contains corrupt state")]
    WebhookReplayCorruptState,

    /// A mock verifier was mounted on the production-safe webhook middleware.
    #[error("Mock webhook mode is not allowed by this middleware: {0}")]
    MockWebhookNotAllowed(String),

    /// Webhook payload parsing failed.
    #[error("Failed to parse webhook payload: {0}")]
    PayloadParseError(String),

    /// Provider API request failed.
    #[error("Payment provider API request failed: {0}")]
    ProviderRequestFailed(String),

    /// Redacted provider transport, HTTP, or response-contract failure.
    #[error(transparent)]
    Provider(#[from] ProviderFailure),

    /// The requested operation is not supported by the payment provider.
    #[error("Operation not supported by provider: {0}")]
    UnsupportedOperation(String),

    /// An error occurred during subscription lifecycle management.
    #[error("Subscription error: {0}")]
    SubscriptionError(String),

    /// A direct-charge request violates a local validation invariant.
    #[error("Invalid charge request: {0}")]
    InvalidCharge(String),

    /// An invoice violates the bounded rendering or money contract.
    #[error("Invalid invoice: {0}")]
    InvalidInvoice(String),

    /// A metered-usage request violates its provider-specific contract.
    #[error("Invalid metered usage: {0}")]
    InvalidUsage(String),

    /// Shared quota validation, reservation, or durable accounting failed.
    #[error(transparent)]
    Quota(#[from] crate::quota::QuotaError),

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

        let usage = CapitalError::InvalidUsage("quantity is zero".to_string());
        assert_eq!(usage.to_string(), "Invalid metered usage: quantity is zero");

        assert_eq!(
            CapitalError::WebhookReplayStoreFull.to_string(),
            "Webhook replay store is full"
        );
        assert_eq!(
            CapitalError::WebhookReplayStoreUnavailable.to_string(),
            "Webhook replay store is unavailable"
        );
        assert_eq!(
            CapitalError::WebhookReplayConfigurationDrift.to_string(),
            "Webhook replay store configuration differs from persisted metadata"
        );
        assert_eq!(
            CapitalError::WebhookReplayCorruptState.to_string(),
            "Webhook replay store contains corrupt state"
        );

        let quota = CapitalError::from(crate::QuotaError::LimitExceeded {
            used: 2,
            requested: 1,
            limit: 2,
        });
        assert_eq!(
            quota.to_string(),
            "quota exceeded: used 2, requested 1, limit 2"
        );

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

    #[test]
    fn provider_failures_are_redacted_and_classified_deterministically() {
        let request = ProviderFailure::request_build("stripe", "checkout");
        assert_eq!(request.class(), ProviderFailureClass::Permanent);
        assert!(!request.is_retryable());

        let transport = ProviderFailure::transport("stripe", "checkout");
        assert_eq!(transport.class(), ProviderFailureClass::Transient);
        assert!(transport.is_retryable());

        let limited = ProviderFailure::http_response(
            "stripe",
            "checkout",
            429,
            Some(Duration::from_secs(30)),
        );
        assert_eq!(limited.class(), ProviderFailureClass::RateLimited);
        assert_eq!(limited.status(), Some(429));
        assert_eq!(limited.retry_after(), Some(Duration::from_secs(30)));
        assert_eq!(limited.to_string(), "stripe checkout returned HTTP 429");
        assert!(!limited.to_string().contains("secret"));

        let unavailable = ProviderFailure::http_response("wise", "payout", 503, None);
        assert_eq!(unavailable.class(), ProviderFailureClass::Transient);
        let rejected = ProviderFailure::http_response("wise", "payout", 422, None);
        assert_eq!(rejected.class(), ProviderFailureClass::Permanent);
        let malformed = ProviderFailure::invalid_response("wise", "payout");
        assert_eq!(malformed.kind(), ProviderFailureKind::InvalidResponse);
        assert_eq!(malformed.provider(), "wise");
        assert_eq!(malformed.operation(), "payout");
        let mismatch = ProviderFailure::contract_mismatch("stripe", "direct charge");
        assert_eq!(mismatch.kind(), ProviderFailureKind::ContractMismatch);
        assert_eq!(mismatch.class(), ProviderFailureClass::Permanent);
    }
}
