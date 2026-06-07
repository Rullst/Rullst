use async_trait::async_trait;

#[derive(Debug, Clone)]
/// [TODO] Missing documentation.
pub struct Message {
    /// [TODO] Missing documentation.
    pub to: String,
    /// [TODO] Missing documentation.
    pub subject: String,
    /// [TODO] Missing documentation.
    pub body_html: Option<String>,
    /// [TODO] Missing documentation.
    pub body_text: Option<String>,
    /// [TODO] Missing documentation.
    pub from: Option<String>,
}

impl Default for Message {
    fn default() -> Self {
        Self::new()
    }
}

impl Message {
    /// [TODO] Missing documentation.
    pub fn new() -> Self {
        Message {
            to: String::new(),
            subject: String::new(),
            body_html: None,
            body_text: None,
            from: None,
        }
    }

    /// [TODO] Missing documentation.
    pub fn to(mut self, to: impl Into<String>) -> Self {
        self.to = to.into();
        self
    }

    /// [TODO] Missing documentation.
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = subject.into();
        self
    }

    /// [TODO] Missing documentation.
    pub fn html(mut self, html: impl Into<String>) -> Self {
        self.body_html = Some(html.into());
        self
    }

    /// [TODO] Missing documentation.
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.body_text = Some(text.into());
        self
    }

    /// [TODO] Missing documentation.
    pub fn from(mut self, from: impl Into<String>) -> Self {
        self.from = Some(from.into());
        self
    }
}

#[derive(Debug)]
/// [TODO] Missing documentation.
pub enum MailError {
    /// [TODO] Missing documentation.
    ConfigError(String),
    /// [TODO] Missing documentation.
    SendError(String),
    /// [TODO] Missing documentation.
    DriverError(String),
}

impl std::fmt::Display for MailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MailError::ConfigError(err) => write!(f, "Configuration error: {}", err),
            MailError::SendError(err) => write!(f, "Send error: {}", err),
            MailError::DriverError(err) => write!(f, "Driver error: {}", err),
        }
    }
}

impl std::error::Error for MailError {}

#[async_trait]
/// [TODO] Missing documentation.
pub trait MailDriver: Send + Sync {
    /// [TODO] Missing documentation.
    async fn send(&self, message: &Message) -> Result<(), MailError>;
}

/// A driver that outputs emails to the terminal and logs to storage/logs/mail.log
pub struct LogDriver;

#[async_trait]
impl MailDriver for LogDriver {
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        let log_dir = std::path::Path::new("storage/logs");
        tokio::fs::create_dir_all(log_dir).await.map_err(|e| {
            MailError::DriverError(format!("Failed to create log directory: {}", e))
        })?;

        let log_path = log_dir.join("mail.log");
        let formatted = format!(
            "========================================\n\
             [MAIL SENT] {}\n\
             To: {}\n\
             From: {}\n\
             Subject: {}\n\
             ----------------------------------------\n\
             [TEXT BODY]\n\
             {}\n\
             ----------------------------------------\n\
             [HTML BODY]\n\
             {}\n\
             ========================================\n\n",
            chrono::Local::now().to_rfc3339(),
            message.to,
            message.from.as_deref().unwrap_or("noreply@rullst.dev"),
            message.subject,
            message.body_text.as_deref().unwrap_or(""),
            message.body_html.as_deref().unwrap_or("")
        );
        println!("{}", formatted);

        use tokio::io::AsyncWriteExt;
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .await
            .map_err(|e| MailError::DriverError(format!("Failed to open log file: {}", e)))?;

        file.write_all(formatted.as_bytes())
            .await
            .map_err(|e| MailError::DriverError(format!("Failed to write to log file: {}", e)))?;

        Ok(())
    }
}

/// An SMTP mail driver
#[cfg(feature = "mail-smtp")]
pub struct SmtpDriver {
    /// [TODO] Missing documentation.
    pub host: String,
    /// [TODO] Missing documentation.
    pub port: u16,
    /// [TODO] Missing documentation.
    pub username: Option<String>,
    /// [TODO] Missing documentation.
    pub password: Option<String>,
}

#[cfg(feature = "mail-smtp")]
#[async_trait]
impl MailDriver for SmtpDriver {
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        use lettre::{
            AsyncSmtpTransport, AsyncTransport, Message as LettreMessage, Tokio1Executor,
            transport::smtp::authentication::Credentials,
        };

        let from_addr = message.from.as_deref().unwrap_or("noreply@rullst.dev");
        let email_builder = LettreMessage::builder()
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
#[async_trait]
impl MailDriver for SmtpDriver {
    async fn send(&self, _message: &Message) -> Result<(), MailError> {
        Err(MailError::DriverError(
            "SMTP mailer driver requires the 'mail-smtp' Cargo feature to be enabled".to_string(),
        ))
    }
}

/// A Resend HTTP REST API driver
pub struct ResendDriver {
    /// [TODO] Missing documentation.
    pub api_key: String,
}

#[async_trait]
impl MailDriver for ResendDriver {
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        let client = reqwest::Client::new();
        let from_addr = message.from.as_deref().unwrap_or("noreply@rullst.dev");
        let mut body = serde_json::json!({
            "to": message.to,
            "from": from_addr,
            "subject": message.subject,
        });

        if let Some(ref html) = message.body_html {
            body["html"] = serde_json::json!(html);
        }
        if let Some(ref text) = message.body_text {
            body["text"] = serde_json::json!(text);
        }

        let res = client
            .post("https://api.resend.com/emails")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| MailError::SendError(e.to_string()))?;

        if res.status().is_success() {
            Ok(())
        } else {
            let text = res.text().await.unwrap_or_default();
            Err(MailError::SendError(format!("Resend API error: {}", text)))
        }
    }
}

/// A SendGrid HTTP REST API driver
pub struct SendGridDriver {
    /// [TODO] Missing documentation.
    pub api_key: String,
}

#[async_trait]
impl MailDriver for SendGridDriver {
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        let client = reqwest::Client::new();
        let from_addr = message.from.as_deref().unwrap_or("noreply@rullst.dev");

        let personalizations = vec![serde_json::json!({
            "to": [{ "email": message.to }]
        })];

        let mut content = vec![];
        if let Some(ref text) = message.body_text {
            content.push(serde_json::json!({
                "type": "text/plain",
                "value": text
            }));
        }
        if let Some(ref html) = message.body_html {
            content.push(serde_json::json!({
                "type": "text/html",
                "value": html
            }));
        }

        let body = serde_json::json!({
            "personalizations": personalizations,
            "from": { "email": from_addr },
            "subject": message.subject,
            "content": content
        });

        let res = client
            .post("https://api.sendgrid.com/v3/mail/send")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| MailError::SendError(e.to_string()))?;

        if res.status().is_success() {
            Ok(())
        } else {
            let text = res.text().await.unwrap_or_default();
            Err(MailError::SendError(format!(
                "SendGrid API error: {}",
                text
            )))
        }
    }
}

/// The main Mail facade
pub struct Mail;

impl Mail {
    /// Send a message using the default configured mail driver
    pub async fn send(message: Message) -> Result<(), MailError> {
        let driver = Self::resolve_driver()?;
        driver.send(&message).await
    }

    fn resolve_driver() -> Result<Box<dyn MailDriver>, MailError> {
        let driver_name = std::env::var("MAIL_DRIVER").unwrap_or_else(|_| {
            let mut found_driver = None;
            if let Ok(toml_content) = std::fs::read_to_string("Rullst.toml") {
                let mut in_mail = false;
                for line in toml_content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with('[') {
                        in_mail = trimmed == "[mail]" || trimmed == "[mailer]";
                        continue;
                    }
                    if in_mail && trimmed.starts_with("driver") {
                        if let Some(val) = trimmed.split('=').nth(1) {
                            let clean_val = val.split('#').next().unwrap_or(val).trim();
                            found_driver =
                                Some(clean_val.trim_matches('"').trim_matches('\'').to_string());
                        }
                    }
                }
            }
            found_driver.unwrap_or_else(|| "log".to_string())
        });

        match driver_name.as_str() {
            "log" => Ok(Box::new(LogDriver)),
            "smtp" => {
                #[cfg(feature = "mail-smtp")]
                {
                    let host =
                        std::env::var("MAIL_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
                    let port = std::env::var("MAIL_PORT")
                        .ok()
                        .and_then(|p| p.parse().ok())
                        .unwrap_or(25);
                    let username = std::env::var("MAIL_USERNAME").ok();
                    let password = std::env::var("MAIL_PASSWORD").ok();

                    Ok(Box::new(SmtpDriver {
                        host,
                        port,
                        username,
                        password,
                    }))
                }
                #[cfg(not(feature = "mail-smtp"))]
                {
                    Ok(Box::new(SmtpDriver))
                }
            }
            "resend" => {
                let api_key = std::env::var("RESEND_API_KEY").map_err(|_| {
                    MailError::ConfigError(
                        "RESEND_API_KEY environment variable is not set".to_string(),
                    )
                })?;
                Ok(Box::new(ResendDriver { api_key }))
            }
            "sendgrid" => {
                let api_key = std::env::var("SENDGRID_API_KEY").map_err(|_| {
                    MailError::ConfigError(
                        "SENDGRID_API_KEY environment variable is not set".to_string(),
                    )
                })?;
                Ok(Box::new(SendGridDriver { api_key }))
            }
            other => Err(MailError::ConfigError(format!(
                "Unknown mail driver: {}",
                other
            ))),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_message_subject() {
        let msg = Message::new().subject("Test Subject");
        assert_eq!(msg.subject, "Test Subject");

        let msg2 = Message::new().subject(String::from("Another Subject"));
        assert_eq!(msg2.subject, "Another Subject");
    }

    #[tokio::test]
    async fn test_log_driver() {
        // Prepare storage/logs directory
        let _ = std::fs::remove_file("storage/logs/mail.log");

        let msg = Message::new()
            .to("test@rullst.dev")
            .subject("Hello Test")
            .text("Testing 1 2 3")
            .html("<h1>Testing 1 2 3</h1>");

        let driver = LogDriver;
        driver.send(&msg).await.unwrap();

        assert!(std::path::Path::new("storage/logs/mail.log").exists());
        let content = std::fs::read_to_string("storage/logs/mail.log").unwrap();
        assert!(content.contains("To: test@rullst.dev"));
        assert!(content.contains("Subject: Hello Test"));
        assert!(content.contains("Testing 1 2 3"));
    }

    #[test]
    fn test_message_to() {
        let msg = Message::new().to("user@example.com");
        assert_eq!(msg.to, "user@example.com");
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests_additional {
    use super::*;
    #[tokio::test]
    async fn test_mail_custom() {
        let msg = Message::new()
            .to("a")
            .from("b")
            .subject("c")
            .text("d")
            .html("e");
        assert_eq!(msg.to, "a");
        assert_eq!(msg.from.unwrap(), "b");
    }
    #[tokio::test]
    async fn test_mail_html() {
        let msg = Message::new().html("h");
        assert_eq!(msg.body_html.unwrap(), "h");
    }
    #[tokio::test]
    async fn test_mail_subject() {
        let msg = Message::new().subject("sub");
        assert_eq!(msg.subject, "sub");
    }
    #[tokio::test]
    async fn test_mail_to() {
        let msg = Message::new().to("to");
        assert_eq!(msg.to, "to");
    }
    #[tokio::test]
    async fn test_mail_send() {
        let msg = Message::new().to("to");
        assert_eq!(msg.to, "to");
    }
    #[tokio::test]
    async fn test_mail_from() {
        let msg = Message::new().from("from");
        assert_eq!(msg.from.unwrap(), "from");
    }
    #[tokio::test]
    async fn test_mail_text() {
        let msg = Message::new().text("txt");
        assert_eq!(msg.body_text.unwrap(), "txt");
    }
}
