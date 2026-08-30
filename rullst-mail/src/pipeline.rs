//! Mandatory pre-flight pipeline shared by every official delivery path.

use crate::error::MailError;
use crate::message::Message;
use crate::security::is_crlf_safe;
use crate::validator::{validate_email_deliverability, validate_email_syntax};

const MAX_TENANT_ID_LEN: usize = 128;

/// Validated tenant metadata associated with a delivery.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DeliveryContext {
    tenant_id: Option<String>,
}

impl DeliveryContext {
    /// Creates a delivery context without tenant routing.
    pub fn global() -> Self {
        Self { tenant_id: None }
    }

    /// Creates a validated tenant-scoped delivery context.
    pub fn for_tenant(tenant_id: impl Into<String>) -> Result<Self, MailError> {
        let tenant_id = tenant_id.into();
        validate_tenant_id(&tenant_id)?;
        Ok(Self {
            tenant_id: Some(tenant_id),
        })
    }

    /// Returns the validated tenant identifier, when present.
    pub fn tenant_id(&self) -> Option<&str> {
        self.tenant_id.as_deref()
    }
}

/// A message which passed header, recipient, content-security, and DLP checks.
#[derive(Debug, Clone)]
pub struct PreparedMessage {
    message: Message,
    context: DeliveryContext,
}

impl PreparedMessage {
    /// Returns the sanitized message ready for transport serialization.
    pub fn message(&self) -> &Message {
        &self.message
    }

    /// Consumes the wrapper and returns its sanitized message.
    pub fn into_message(self) -> Message {
        self.message
    }

    /// Returns the validated delivery context.
    pub fn context(&self) -> &DeliveryContext {
        &self.context
    }
}

/// Stateless mandatory delivery pipeline.
pub struct DeliveryPipeline;

impl DeliveryPipeline {
    /// Validates and sanitizes a global message before queueing or transport delivery.
    pub fn prepare(message: &Message) -> Result<PreparedMessage, MailError> {
        Self::prepare_with_context(message, DeliveryContext::global())
    }

    /// Validates and sanitizes a tenant-scoped message before queueing or delivery.
    pub fn prepare_for_tenant(
        tenant_id: impl Into<String>,
        message: &Message,
    ) -> Result<PreparedMessage, MailError> {
        Self::prepare_with_context(message, DeliveryContext::for_tenant(tenant_id)?)
    }

    fn prepare_with_context(
        message: &Message,
        context: DeliveryContext,
    ) -> Result<PreparedMessage, MailError> {
        validate_header("To", &message.to)?;
        validate_header("Subject", &message.subject)?;
        validate_email_deliverability(&message.to)
            .map_err(|error| MailError::ValidationError(error.to_string()))?;

        if let Some(from) = message.from.as_deref() {
            validate_header("From", from)?;
            validate_email_syntax(from)
                .map_err(|error| MailError::ValidationError(error.to_string()))?;
        }

        if let Some(email) = message.unsubscribe_email.as_deref() {
            validate_header("List-Unsubscribe email", email)?;
            validate_email_syntax(email)
                .map_err(|error| MailError::ValidationError(error.to_string()))?;
        }

        if let Some(url) = message.unsubscribe_url.as_deref() {
            validate_header("List-Unsubscribe URL", url)?;
            validate_http_url("List-Unsubscribe URL", url)?;
        }

        for attachment in &message.attachments {
            validate_header("attachment filename", &attachment.filename)?;
            validate_header("attachment MIME type", &attachment.mime_type)?;
            if let Some(cid) = attachment.cid.as_deref() {
                validate_header("attachment Content-ID", cid)?;
            }
        }

        message.validate_security()?;

        Ok(PreparedMessage {
            message: message.clone().sanitize_secrets(),
            context,
        })
    }
}

/// Validates an application-owned HTTP(S) action URL before it is embedded in a message.
///
/// URL credentials and header-breaking characters are rejected. Redirect policy, destination
/// authorization, and the behavior of the application endpoint remain host responsibilities.
pub fn validate_action_url(value: impl Into<String>) -> Result<(), MailError> {
    let value = value.into();
    validate_header("Action URL", &value)?;
    let url = reqwest::Url::parse(&value)
        .map_err(|_| MailError::ValidationError("Action URL is not a valid URL".to_string()))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(MailError::ValidationError(
            "Action URL must use HTTP or HTTPS and include a host".to_string(),
        ));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(MailError::ValidationError(
            "Action URL must not contain embedded credentials".to_string(),
        ));
    }
    Ok(())
}

fn validate_header(name: &str, value: &str) -> Result<(), MailError> {
    if is_crlf_safe(value) {
        Ok(())
    } else {
        Err(MailError::ValidationError(format!(
            "{name} contains forbidden CR/LF characters"
        )))
    }
}

fn validate_http_url(name: &str, value: &str) -> Result<(), MailError> {
    let url = reqwest::Url::parse(value)
        .map_err(|_| MailError::ValidationError(format!("{name} is not a valid URL")))?;
    if matches!(url.scheme(), "http" | "https") && url.host_str().is_some() {
        Ok(())
    } else {
        Err(MailError::ValidationError(format!(
            "{name} must use HTTP or HTTPS and include a host"
        )))
    }
}

fn validate_tenant_id(tenant_id: &str) -> Result<(), MailError> {
    if tenant_id.is_empty() || tenant_id.len() > MAX_TENANT_ID_LEN {
        return Err(MailError::ValidationError(format!(
            "tenant ID must contain between 1 and {MAX_TENANT_ID_LEN} bytes"
        )));
    }
    if !tenant_id
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(MailError::ValidationError(
            "tenant ID may contain only ASCII letters, digits, '-' and '_'".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_crlf_disposable_recipient_and_dangerous_link() {
        let crlf = Message::new()
            .to("victim@example.com\r\nBcc: attacker@example.com")
            .text("safe");
        assert!(matches!(
            DeliveryPipeline::prepare(&crlf),
            Err(MailError::ValidationError(_))
        ));

        let disposable = Message::new().to("test@mailinator.com").text("safe");
        assert!(matches!(
            DeliveryPipeline::prepare(&disposable),
            Err(MailError::ValidationError(_))
        ));

        let dangerous = Message::new()
            .to("valid@example.com")
            .html(r#"<a href="javascript:alert(1)">click</a>"#);
        assert!(matches!(
            DeliveryPipeline::prepare(&dangerous),
            Err(MailError::SendError(_))
        ));
    }

    #[test]
    fn sanitizes_secrets_and_validates_tenant() {
        let message = Message::new()
            .to("valid@example.com")
            .text("password=hunter2");
        let prepared = DeliveryPipeline::prepare_for_tenant("acme_42", &message)
            .expect("valid tenant-scoped message");
        assert_eq!(prepared.context().tenant_id(), Some("acme_42"));
        assert_eq!(
            prepared.message().body_text.as_deref(),
            Some("password=[REDACTED]")
        );

        assert!(DeliveryPipeline::prepare_for_tenant("../acme", &message).is_err());
    }

    #[test]
    fn action_urls_require_http_authority_without_credentials() {
        assert!(validate_action_url("https://example.com/billing?id=42").is_ok());
        for invalid in [
            "javascript:alert(1)",
            "ftp://example.com/file",
            "https://",
            "https://user:secret@example.com/billing",
            "https://example.com/ok\r\nBcc: attacker@example.com",
        ] {
            assert!(validate_action_url(invalid).is_err(), "accepted {invalid}");
        }
    }
}
