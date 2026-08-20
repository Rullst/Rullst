// src/drivers/smtp.rs — Native async SMTP driver with RFC 8058 support.

use crate::drivers::MailDriver;
use crate::error::MailError;
use crate::message::Message;
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
#[async_trait]
impl MailDriver for SmtpDriver {
    #[cfg_attr(mutants, mutants::skip)]
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        use lettre::{
            AsyncSmtpTransport, AsyncTransport, Message as LettreMessage, Tokio1Executor,
            transport::smtp::authentication::Credentials,
        };

        let from_addr = message.from.as_deref().unwrap_or("noreply@rullst.dev");
        let mut email_builder = LettreMessage::builder()
            .from(
                from_addr
                    .parse()
                    .map_err(|e| MailError::SendError(format!("{}", e)))?,
            )
            .to(message
                .to
                .parse()
                .map_err(|e| MailError::SendError(format!("{}", e)))?)
            .subject(&message.subject);

        if let Some(unsub) = message.list_unsubscribe_header() {
            let header =
                lettre::message::header::HeaderName::new_from_ascii_str("List-Unsubscribe");
            email_builder =
                email_builder.raw_header(lettre::message::header::HeaderValue::new(header, unsub));
            if message.unsubscribe_url.is_some() {
                let header = lettre::message::header::HeaderName::new_from_ascii_str(
                    "List-Unsubscribe-Post",
                );
                email_builder =
                    email_builder.raw_header(lettre::message::header::HeaderValue::new(
                        header,
                        "List-Unsubscribe=One-Click".to_string(),
                    ));
            }
        }

        let email = if let Some(ref html) = message.body_html {
            if let Some(ref text) = message.body_text {
                email_builder
                    .multipart(
                        lettre::message::MultiPart::alternative()
                            .singlepart(lettre::message::SinglePart::plain(text.clone()))
                            .singlepart(lettre::message::SinglePart::html(html.clone())),
                    )
                    .map_err(|e| MailError::SendError(format!("{}", e)))?
            } else {
                email_builder
                    .header(lettre::message::header::ContentType::TEXT_HTML)
                    .body(html.clone())
                    .map_err(|e| MailError::SendError(format!("{}", e)))?
            }
        } else if let Some(ref text) = message.body_text {
            email_builder
                .header(lettre::message::header::ContentType::TEXT_PLAIN)
                .body(text.clone())
                .map_err(|e| MailError::SendError(format!("{}", e)))?
        } else {
            return Err(MailError::SendError("No email body provided".to_string()));
        };

        let mut builder = AsyncSmtpTransport::<Tokio1Executor>::relay(&self.host)
            .map_err(|e| MailError::SendError(e.to_string()))?
            .port(self.port);

        if let (Some(user), Some(pass)) = (&self.username, &self.password) {
            builder = builder.credentials(Credentials::new(user.clone(), pass.clone()));
        }

        let transport = builder.build();
        transport
            .send(email)
            .await
            .map_err(|e| MailError::SendError(format!("{}", e)))?;
        Ok(())
    }
}

/// Placeholder SMTP driver if Cargo feature is not enabled
#[cfg(not(feature = "mail-smtp"))]
pub struct SmtpDriver;

#[cfg(not(feature = "mail-smtp"))]
#[cfg_attr(mutants, mutants::skip)]
#[async_trait]
impl MailDriver for SmtpDriver {
    async fn send(&self, _message: &Message) -> Result<(), MailError> {
        Err(MailError::DriverError(
            "SMTP mailer driver requires the 'mail-smtp' Cargo feature to be enabled".to_string(),
        ))
    }
}
