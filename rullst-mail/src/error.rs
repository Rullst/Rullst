// src/error.rs — Core error definitions for rullst-mail.

use std::time::Duration;

/// Operational disposition used by retry, failover, and telemetry policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MailFailureClass {
    /// The same message/configuration must not be retried against another provider.
    Permanent,
    /// A transport or provider availability failure may be retried or failed over.
    Transient,
    /// The provider explicitly asked the sender to slow down.
    RateLimited,
}

impl MailFailureClass {
    /// Stable low-cardinality label suitable for telemetry fields.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Permanent => "permanent",
            Self::Transient => "transient",
            Self::RateLimited => "rate_limited",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Errors that can occur during mail operations.
#[non_exhaustive]
pub enum MailError {
    /// Configuration errors (e.g. missing API keys).
    ConfigError(String),
    /// Errors occurred while sending the message.
    SendError(String),
    /// Errors related to the driver backend itself.
    DriverError(String),
    /// A message or tenant context failed the mandatory pre-flight checks.
    ValidationError(String),
    /// The provider returned a typed non-rate-limit HTTP response.
    ProviderResponse {
        /// Stable provider label.
        provider: &'static str,
        /// HTTP response status.
        status: u16,
        /// Provider response detail; official adapters bound and secret-redact this field.
        message: String,
    },
    /// The provider could not be reached or its transport failed before a response.
    TransportError {
        /// Stable provider/transport label.
        provider: &'static str,
        /// Operational detail; official adapters use a bounded non-secret value.
        message: String,
    },
    /// The provider rejected delivery because its current rate budget was exhausted.
    RateLimited {
        /// Stable provider label.
        provider: &'static str,
        /// Provider response detail; official adapters bound and secret-redact this field.
        message: String,
        /// Delta-seconds `Retry-After`, capped at one day when supplied.
        retry_after: Option<Duration>,
    },
    /// Delivery was blocked before transport by authoritative suppression state.
    SuppressedRecipient {
        /// Stable reason label; the recipient address is deliberately omitted.
        reason: &'static str,
    },
    /// Suppression state could not be checked, so delivery failed closed.
    SuppressionUnavailable,
    /// An attachment was rejected before any transport received it.
    AttachmentRejected {
        /// Stable reason label; filename and content are deliberately omitted.
        reason: &'static str,
    },
    /// Attachment inspection could not complete, so delivery failed closed.
    AttachmentInspectionUnavailable,
}

impl MailError {
    /// Classifies the error for deterministic failover and retry decisions.
    pub const fn failure_class(&self) -> MailFailureClass {
        match self {
            Self::TransportError { .. } | Self::DriverError(_) => MailFailureClass::Transient,
            Self::RateLimited { .. } => MailFailureClass::RateLimited,
            Self::ProviderResponse { status, .. } if *status >= 500 => MailFailureClass::Transient,
            Self::ConfigError(_)
            | Self::SendError(_)
            | Self::ValidationError(_)
            | Self::SuppressedRecipient { .. }
            | Self::SuppressionUnavailable
            | Self::AttachmentRejected { .. }
            | Self::AttachmentInspectionUnavailable
            | Self::ProviderResponse { .. } => MailFailureClass::Permanent,
        }
    }

    /// Returns whether another provider may safely receive this already validated message.
    pub const fn is_failover_eligible(&self) -> bool {
        matches!(
            self.failure_class(),
            MailFailureClass::Transient | MailFailureClass::RateLimited
        )
    }

    /// Returns a bounded provider response error with typed HTTP classification.
    pub fn from_provider_response(
        provider: &'static str,
        status: u16,
        message: impl Into<String>,
        retry_after: Option<Duration>,
    ) -> Self {
        let message = bounded_message(message.into());
        if status == 429 {
            Self::RateLimited {
                provider,
                message,
                retry_after,
            }
        } else {
            Self::ProviderResponse {
                provider,
                status,
                message,
            }
        }
    }

    /// Returns a bounded transient transport failure without exposing raw client errors.
    pub fn transport(provider: &'static str, message: impl Into<String>) -> Self {
        Self::TransportError {
            provider,
            message: bounded_message(message.into()),
        }
    }

    /// Returns the provider retry delay when a bounded delta-seconds value was supplied.
    pub const fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::RateLimited { retry_after, .. } => *retry_after,
            _ => None,
        }
    }
}

impl std::fmt::Display for MailError {
    #[cfg_attr(mutants, mutants::skip)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MailError::ConfigError(err) => write!(f, "Configuration error: {}", err),
            MailError::SendError(err) => write!(f, "Send error: {}", err),
            MailError::DriverError(err) => write!(f, "Driver error: {}", err),
            MailError::ValidationError(err) => write!(f, "Validation error: {}", err),
            MailError::ProviderResponse {
                provider,
                status,
                message,
            } => write!(f, "{provider} rejected mail with HTTP {status}: {message}"),
            MailError::TransportError { provider, message } => {
                write!(f, "{provider} transport error: {message}")
            }
            MailError::RateLimited {
                provider,
                message,
                retry_after,
            } => {
                write!(f, "{provider} rate limit: {message}")?;
                if let Some(delay) = retry_after {
                    write!(f, " (retry after {}s)", delay.as_secs())?;
                }
                Ok(())
            }
            MailError::SuppressedRecipient { reason } => {
                write!(f, "Recipient is suppressed ({reason})")
            }
            MailError::SuppressionUnavailable => {
                write!(f, "Suppression state is unavailable; delivery blocked")
            }
            MailError::AttachmentRejected { reason } => {
                write!(f, "Attachment rejected before delivery ({reason})")
            }
            MailError::AttachmentInspectionUnavailable => {
                write!(f, "Attachment inspection is unavailable; delivery blocked")
            }
        }
    }
}

impl std::error::Error for MailError {}

const MAX_ERROR_MESSAGE_LEN: usize = 4_096;

fn bounded_message(mut message: String) -> String {
    if message.len() > MAX_ERROR_MESSAGE_LEN {
        let mut boundary = MAX_ERROR_MESSAGE_LEN;
        while boundary > 0 && !message.is_char_boundary(boundary) {
            boundary -= 1;
        }
        message.truncate(boundary);
        message.push_str("…[truncated]");
    }
    message
}

pub(crate) async fn provider_http_error(
    provider: &'static str,
    mut response: reqwest::Response,
) -> MailError {
    let status = response.status().as_u16();
    let retry_after = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map(|seconds| Duration::from_secs(seconds.min(86_400)));
    let mut body = Vec::new();
    while body.len() < MAX_ERROR_MESSAGE_LEN {
        match response.chunk().await {
            Ok(Some(chunk)) => {
                let remaining = MAX_ERROR_MESSAGE_LEN - body.len();
                body.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
                if chunk.len() > remaining {
                    break;
                }
            }
            Ok(None) => break,
            Err(_) => return MailError::transport(provider, "failed to read provider response"),
        }
    }
    let detail = crate::security::redact_email_secrets(&String::from_utf8_lossy(&body));
    MailError::from_provider_response(provider, status, detail, retry_after)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[tokio::test]
    async fn http_rate_limit_is_bounded_redacted_and_typed() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind HTTP fixture");
        let address = listener.local_addr().expect("fixture address");
        let fixture = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().expect("accept fixture request");
            let mut request = [0_u8; 1_024];
            let _ = socket.read(&mut request).expect("read fixture request");
            let body = "password=provider_secret";
            write!(
                socket,
                "HTTP/1.1 429 Too Many Requests\r\nRetry-After: 999999\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .expect("write fixture response");
        });
        let response = reqwest::get(format!("http://{address}/mail"))
            .await
            .expect("fixture response");
        let error = provider_http_error("fixture", response).await;

        assert_eq!(error.failure_class(), MailFailureClass::RateLimited);
        assert_eq!(error.retry_after(), Some(Duration::from_secs(86_400)));
        assert!(!error.to_string().contains("provider_secret"));
        assert!(error.to_string().contains("[REDACTED]"));
        fixture.join().expect("HTTP fixture");
    }
}
