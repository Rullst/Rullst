// src/drivers/smtp.rs — Native async SMTP driver with RFC 8058 support.

use super::traits::MailDriver;
#[cfg(feature = "mail-smtp")]
use super::{DeliveryMode, credential_mode};
#[cfg(feature = "mail-smtp")]
use crate::drivers::mock::{record_offline_delivery, validate_credential};
use crate::error::MailError;
use crate::message::Message;
#[cfg(feature = "mail-smtp")]
use crate::pipeline::DeliveryPipeline;
use async_trait::async_trait;

/// An SMTP mail driver
#[cfg(feature = "mail-smtp")]
pub struct SmtpDriver {
    /// SMTP server hostname or IP.
    pub host: String,
    /// SMTP port (e.g. 587, 465, 25).
    pub port: u16,
    /// Optional username for authentication.
    pub username: Option<String>,
    /// Optional password for authentication.
    pub password: Option<String>,
}

#[cfg(feature = "mail-smtp")]
impl SmtpDriver {
    /// Creates an SMTP driver after validating its configuration.
    pub fn try_new(
        host: impl Into<String>,
        port: u16,
        username: Option<String>,
        password: Option<String>,
    ) -> Result<Self, MailError> {
        let driver = Self {
            host: host.into(),
            port,
            username,
            password,
        };
        driver.validate_config()?;
        Ok(driver)
    }

    /// Returns whether SMTP or the deterministic offline fallback will be used.
    pub fn delivery_mode(&self) -> DeliveryMode {
        if self.host.trim().is_empty() || credential_mode(&self.host) == DeliveryMode::OfflineMock {
            return DeliveryMode::OfflineMock;
        }
        match (self.username.as_deref(), self.password.as_deref()) {
            (Some(user), Some(password))
                if credential_mode(user) == DeliveryMode::Real
                    && credential_mode(password) == DeliveryMode::Real =>
            {
                DeliveryMode::Real
            }
            _ => DeliveryMode::OfflineMock,
        }
    }

    fn validate_config(&self) -> Result<(), MailError> {
        validate_credential("SMTP host", &self.host)?;
        if self.port == 0 {
            return Err(MailError::ConfigError(
                "SMTP port must be greater than zero".to_string(),
            ));
        }
        if let Some(username) = self.username.as_deref() {
            validate_credential("SMTP username", username)?;
        }
        if let Some(password) = self.password.as_deref() {
            validate_credential("SMTP password", password)?;
        }
        Ok(())
    }
}

#[cfg(feature = "mail-smtp")]
#[async_trait]
impl MailDriver for SmtpDriver {
    #[cfg_attr(mutants, mutants::skip)]
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        use lettre::{
            AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
            transport::smtp::authentication::Credentials,
        };

        self.validate_config()?;
        let prepared = DeliveryPipeline::prepare(message)?;
        let message = prepared.message();
        if self.delivery_mode() == DeliveryMode::OfflineMock {
            return record_offline_delivery("smtp", message);
        }
        DeliveryPipeline::require_due("SMTP", message)?;

        let email = build_smtp_message(message)?;

        let mut builder = AsyncSmtpTransport::<Tokio1Executor>::relay(&self.host)
            .map_err(|_| MailError::ConfigError("SMTP relay configuration is invalid".to_string()))?
            .port(self.port);

        if let (Some(user), Some(pass)) = (&self.username, &self.password) {
            builder = builder.credentials(Credentials::new(user.clone(), pass.clone()));
        }

        let transport = builder.build();
        transport.send(email).await.map_err(classify_smtp_error)?;
        Ok(())
    }
}

#[cfg(feature = "mail-smtp")]
enum SmtpBody {
    Single(lettre::message::SinglePart),
    Multi(lettre::message::MultiPart),
}

#[cfg(feature = "mail-smtp")]
fn build_smtp_message(message: &Message) -> Result<lettre::Message, MailError> {
    use lettre::Message as LettreMessage;

    let from_addr = message.from.as_deref().unwrap_or("noreply@rullst.dev");
    let mut builder = LettreMessage::builder()
        .from(
            from_addr
                .parse()
                .map_err(|error| MailError::ValidationError(format!("invalid sender: {error}")))?,
        )
        .to(message
            .to
            .parse()
            .map_err(|error| MailError::ValidationError(format!("invalid recipient: {error}")))?)
        .subject(&message.subject);

    if let Some(unsubscribe) = message.list_unsubscribe_header() {
        let header = lettre::message::header::HeaderName::new_from_ascii_str("List-Unsubscribe");
        builder = builder.raw_header(lettre::message::header::HeaderValue::new(
            header,
            unsubscribe,
        ));
        if message.unsubscribe_url.is_some() {
            let header =
                lettre::message::header::HeaderName::new_from_ascii_str("List-Unsubscribe-Post");
            builder = builder.raw_header(lettre::message::header::HeaderValue::new(
                header,
                "List-Unsubscribe=One-Click".to_string(),
            ));
        }
    }

    match build_smtp_body(message)? {
        SmtpBody::Single(part) => builder.singlepart(part),
        SmtpBody::Multi(part) => builder.multipart(part),
    }
    .map_err(|error| MailError::ValidationError(error.to_string()))
}

#[cfg(feature = "mail-smtp")]
fn build_smtp_body(message: &Message) -> Result<SmtpBody, MailError> {
    use lettre::message::{MultiPart, SinglePart};

    let inline: Vec<_> = message
        .attachments
        .iter()
        .filter(|attachment| attachment.is_inline())
        .collect();
    let mut content = match (&message.body_text, &message.body_html) {
        (Some(text), Some(html)) if !inline.is_empty() => {
            let related = inline.iter().try_fold(
                MultiPart::related().singlepart(SinglePart::html(html.clone())),
                |related, attachment| {
                    Ok::<_, MailError>(related.singlepart(smtp_attachment_part(attachment)?))
                },
            )?;
            SmtpBody::Multi(
                MultiPart::alternative()
                    .singlepart(SinglePart::plain(text.clone()))
                    .multipart(related),
            )
        }
        (Some(text), Some(html)) => SmtpBody::Multi(
            MultiPart::alternative()
                .singlepart(SinglePart::plain(text.clone()))
                .singlepart(SinglePart::html(html.clone())),
        ),
        (None, Some(html)) if !inline.is_empty() => SmtpBody::Multi(inline.iter().try_fold(
            MultiPart::related().singlepart(SinglePart::html(html.clone())),
            |related, attachment| {
                Ok::<_, MailError>(related.singlepart(smtp_attachment_part(attachment)?))
            },
        )?),
        (None, Some(html)) => SmtpBody::Single(SinglePart::html(html.clone())),
        (Some(text), None) => SmtpBody::Single(SinglePart::plain(text.clone())),
        (None, None) => {
            return Err(MailError::ValidationError(
                "No email body provided".to_string(),
            ));
        }
    };

    let regular: Vec<_> = message
        .attachments
        .iter()
        .filter(|attachment| !attachment.is_inline())
        .collect();
    if regular.is_empty() {
        return Ok(content);
    }

    let mut mixed = MultiPart::mixed().build();
    mixed = match content {
        SmtpBody::Single(part) => mixed.singlepart(part),
        SmtpBody::Multi(part) => mixed.multipart(part),
    };
    for attachment in regular {
        mixed = mixed.singlepart(smtp_attachment_part(attachment)?);
    }
    content = SmtpBody::Multi(mixed);
    Ok(content)
}

#[cfg(feature = "mail-smtp")]
fn smtp_attachment_part(
    attachment: &crate::attachment::Attachment,
) -> Result<lettre::message::SinglePart, MailError> {
    use lettre::message::{Attachment as LettreAttachment, header::ContentType};

    let content_type = ContentType::parse(&attachment.mime_type).map_err(|_| {
        MailError::ValidationError("attachment MIME type is invalid for SMTP".to_string())
    })?;
    let builder = match attachment.cid.as_deref() {
        Some(cid) => {
            LettreAttachment::new_inline_with_name(cid.to_string(), attachment.filename.clone())
        }
        None => LettreAttachment::new(attachment.filename.clone()),
    };
    Ok(builder.body(attachment.content.clone(), content_type))
}

#[cfg(feature = "mail-smtp")]
fn classify_smtp_error(error: lettre::transport::smtp::Error) -> MailError {
    if error.is_permanent() {
        MailError::SendError("SMTP server permanently rejected the message".to_string())
    } else if error.is_transient() {
        MailError::transport("smtp", "SMTP server returned a transient response")
    } else {
        MailError::transport("smtp", "SMTP transport failed before accepted delivery")
    }
}

#[cfg(all(test, feature = "mail-smtp"))]
mod tests {
    use super::build_smtp_message;
    use crate::{DeliveryPipeline, MailError, Message};

    #[test]
    fn smtp_mime_nests_alternative_related_and_mixed_parts() {
        let message = Message::new()
            .to("alice@example.com")
            .from("sender@example.com")
            .subject("MIME contract")
            .html("<p>Hello</p><img src=\"cid:brand_logo\">")
            .text("Hello")
            .attach_cid("brand_logo", "logo.png", vec![0, 255, 1, 254], "image/png")
            .attach_bytes(
                "terms.bin",
                vec![2, 253, 3, 252],
                "application/octet-stream",
            );
        let prepared = DeliveryPipeline::prepare(&message).expect("valid message");
        let formatted = String::from_utf8_lossy(
            &build_smtp_message(prepared.message())
                .expect("valid MIME")
                .formatted(),
        )
        .into_owned();

        assert!(formatted.contains("Content-Type: multipart/mixed"));
        assert!(formatted.contains("Content-Type: multipart/alternative"));
        assert!(formatted.contains("Content-Type: multipart/related"));
        assert!(formatted.contains("Content-ID: <brand_logo>"));
        assert!(formatted.contains("Content-Disposition: inline; filename=\"logo.png\""));
        assert!(formatted.contains("Content-Disposition: attachment; filename=\"terms.bin\""));
        assert_eq!(
            formatted
                .matches("Content-Transfer-Encoding: base64")
                .count(),
            2
        );
        assert!(formatted.contains("AP8B/g=="));
        assert!(formatted.contains("Av0D/A=="));
    }

    #[test]
    fn smtp_builder_rejects_a_missing_body_without_panicking() {
        let message = Message::new().to("alice@example.com").subject("No body");
        assert!(matches!(
            build_smtp_message(&message),
            Err(MailError::ValidationError(message)) if message == "No email body provided"
        ));
    }
}

#[cfg(all(test, feature = "mail-smtp"))]
#[path = "smtp_tests.rs"]
mod contract_tests;

/// Placeholder SMTP driver if Cargo feature is not enabled
#[cfg(not(feature = "mail-smtp"))]
pub struct SmtpDriver;

#[cfg(not(feature = "mail-smtp"))]
#[cfg_attr(mutants, mutants::skip)]
#[async_trait]
impl MailDriver for SmtpDriver {
    async fn send(&self, _message: &Message) -> Result<(), MailError> {
        Err(MailError::ConfigError(
            "SMTP mailer driver requires the 'mail-smtp' Cargo feature to be enabled".to_string(),
        ))
    }
}
