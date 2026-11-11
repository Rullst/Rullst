//! The errors of AI providers, guardrails and response parsing.
//!
//! No public item names a `reqwest` or `serde_json` type: transport and JSON
//! failures are wrapped in [`AiRequestError`] and [`AiJsonError`], which keep
//! the library error private and expose it only as the `source()` chain.

use super::VisionInputError;
use std::fmt;
use std::time::Duration;

/// Errors returned by AI providers, guardrails, and response parsing.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum AiError {
    /// A network request failed before a provider response was received.
    #[error("Request error: {0}")]
    RequestError(#[source] AiRequestError),
    /// JSON serialization or deserialization failed.
    #[error("Serialization error: {0}")]
    SerializationError(#[source] AiJsonError),
    /// The provider answered with an unsuccessful HTTP status. The text is
    /// `API error: <Provider> returned HTTP <code> <reason>`, followed by
    /// `: <provider message>` when the response body carried one.
    #[error("API error: {0}")]
    HttpStatus(AiHttpError),
    /// The provider returned a malformed or unusable response.
    #[error("API error: {0}")]
    ApiError(String),
    /// Provider or client configuration is invalid.
    #[error("Configuration error: {0}")]
    ConfigError(String),
    /// Mandatory prompt-injection guardrails rejected a request.
    #[error("Blocked by AI Firewall: {0}")]
    BlockedByFirewall(String),
    /// A message used a role that the portable provider interface cannot represent safely.
    #[error("Invalid AI message role: {0}")]
    InvalidMessageRole(String),
    /// A JSON Schema descriptor is invalid or unsupported by the deterministic mock.
    #[error("Invalid structured output schema: {0}")]
    InvalidSchema(String),
    /// A provider does not implement a capability without unsafe emulation.
    #[error("Provider '{provider}' does not support {capability}")]
    UnsupportedCapability {
        /// Stable provider name.
        provider: &'static str,
        /// Capability that was requested.
        capability: &'static str,
    },
    /// A caller explicitly cancelled an in-flight AI operation.
    #[error("AI operation was cancelled")]
    Cancelled,
    /// A streaming transport violated the bounded protocol contract.
    #[error("AI stream rejected: {0}")]
    StreamProtocol(&'static str),
    /// The application-provided streaming sink rejected an output chunk.
    #[error("AI stream sink rejected a chunk")]
    StreamSink,
    /// A local or remote vision source failed its explicit intake policy.
    #[error("Vision input error: {0}")]
    VisionInput(#[from] VisionInputError),
    /// A legacy catch-all error.
    #[error("Error: {0}")]
    Other(String),
}

impl AiError {
    /// A transport failure, without the request URL (it can carry a key).
    pub(crate) fn request(error: reqwest::Error) -> Self {
        Self::RequestError(AiRequestError(error.without_url()))
    }

    pub(crate) fn json(error: serde_json::Error) -> Self {
        Self::SerializationError(AiJsonError(error))
    }
}

/// A provider request that failed before a response arrived: the connection,
/// TLS, a timeout or reading the body.
///
/// `Display` is the HTTP client's message, without the request URL; the
/// client's own causes follow as [`source`](std::error::Error::source).
#[derive(Debug)]
pub struct AiRequestError(reqwest::Error);

impl AiRequestError {
    /// Whether the request timed out (see `with_request_timeout` on the
    /// providers).
    pub fn is_timeout(&self) -> bool {
        self.0.is_timeout()
    }

    /// Whether the connection to the provider could not be established.
    pub fn is_connect(&self) -> bool {
        self.0.is_connect()
    }
}

impl fmt::Display for AiRequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl std::error::Error for AiRequestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

/// A JSON encoding or decoding failure. `Display` is the JSON library's
/// message, which names the line and column of a syntax error.
#[derive(Debug)]
pub struct AiJsonError(serde_json::Error);

impl fmt::Display for AiJsonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl std::error::Error for AiJsonError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

/// An unsuccessful HTTP status from an AI provider, inside
/// [`AiError::HttpStatus`].
///
/// The built-in providers fill it from the response: the status code, the
/// `Retry-After` delay and the provider's own error code and message from the
/// body, bounded to one line of at most 300 characters with URL queries and
/// key-shaped tokens redacted. `Display` is
/// `<Provider> returned HTTP <code> <reason>`, then `: <code>: <message>` or
/// `: <message>` when the body carried a message.
///
/// ```
/// use rullst_ai::{AiError, AiHttpError};
/// use std::time::Duration;
///
/// let error = AiError::HttpStatus(
///     AiHttpError::new("Gemini", 429)
///         .with_retry_after(Duration::from_secs(52))
///         .with_provider_message(Some("RESOURCE_EXHAUSTED"), "Quota exceeded"),
/// );
/// assert_eq!(
///     error.to_string(),
///     "API error: Gemini returned HTTP 429 Too Many Requests: RESOURCE_EXHAUSTED: Quota exceeded"
/// );
/// if let AiError::HttpStatus(http) = &error {
///     assert_eq!(http.status(), 429);
///     assert_eq!(http.retry_after(), Some(Duration::from_secs(52)));
///     assert_eq!(http.code(), Some("RESOURCE_EXHAUSTED"));
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct AiHttpError {
    provider: String,
    status: u16,
    retry_after: Option<Duration>,
    code: Option<String>,
    message: Option<String>,
}

impl AiHttpError {
    /// An error for `status` from `provider`, the name shown in the text
    /// (`"Gemini"`, `"OpenAI"`, ...), without a delay or provider message.
    pub fn new(provider: impl Into<String>, status: u16) -> Self {
        Self {
            provider: provider.into(),
            status,
            retry_after: None,
            code: None,
            message: None,
        }
    }

    /// Sets how long the provider asked to wait before retrying.
    pub fn with_retry_after(mut self, delay: Duration) -> Self {
        self.retry_after = Some(delay);
        self
    }

    /// Sets the provider's error message and, when it sent one, its error
    /// code (for example `RESOURCE_EXHAUSTED` or `insufficient_quota`).
    pub fn with_provider_message(
        mut self,
        code: Option<impl Into<String>>,
        message: impl Into<String>,
    ) -> Self {
        self.code = code.map(Into::into);
        self.message = Some(message.into());
        self
    }

    /// The provider name shown in the text.
    pub fn provider(&self) -> &str {
        &self.provider
    }

    /// The HTTP status code, for example `429` or `404`.
    pub fn status(&self) -> u16 {
        self.status
    }

    /// The delay the provider asked for: the `Retry-After` header in
    /// seconds or, for Gemini, the `retryDelay` of the body's `RetryInfo`.
    pub fn retry_after(&self) -> Option<Duration> {
        self.retry_after
    }

    /// The provider's error code from the body (`error.status`, `error.code`
    /// or `error.type`), when it sent one.
    pub fn code(&self) -> Option<&str> {
        self.code.as_deref()
    }

    /// The provider's error message from the body, when it sent one.
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}

impl fmt::Display for AiHttpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} returned HTTP {}", self.provider, self.status)?;
        if let Some(reason) = reqwest::StatusCode::from_u16(self.status)
            .ok()
            .and_then(|status| status.canonical_reason())
        {
            write!(formatter, " {reason}")?;
        }
        match (&self.code, &self.message) {
            (Some(code), Some(message)) => write!(formatter, ": {code}: {message}"),
            (None, Some(message)) => write!(formatter, ": {message}"),
            _ => Ok(()),
        }
    }
}

impl std::error::Error for AiHttpError {}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
