// src/drivers/postmark.rs — Postmark HTTP REST API driver with RFC 8058 headers.

use super::traits::MailDriver;
use super::{DeliveryMode, credential_mode};
use crate::drivers::mock::{record_offline_delivery, validate_credential};
use crate::error::MailError;
use crate::message::Message;
use crate::pipeline::DeliveryPipeline;
use async_trait::async_trait;

/// A Postmark HTTP REST API driver.
pub struct PostmarkDriver {
    /// Postmark server API token (`X-Postmark-Server-Token`).
    pub server_token: String,
    /// Optional custom message stream (defaults to `"outbound"` in Postmark).
    pub message_stream: Option<String>,
}

impl PostmarkDriver {
    /// Creates a driver after validating the credential's structural safety.
    pub fn try_new(server_token: impl Into<String>) -> Result<Self, MailError> {
        let server_token = server_token.into();
        validate_credential("Postmark server token", &server_token)?;
        Ok(Self {
            server_token,
            message_stream: None,
        })
    }

    /// Creates a new `PostmarkDriver` with the given server API token.
    ///
    /// Prefer [`Self::try_new`]. Invalid configuration still fails closed in `send`.
    #[deprecated(since = "12.0.0", note = "use PostmarkDriver::try_new")]
    pub fn new(server_token: impl Into<String>) -> Self {
        Self {
            server_token: server_token.into(),
            message_stream: None,
        }
    }

    /// Sets a custom message stream (e.g. `"broadcast"` or `"outbound"`).
    pub fn with_message_stream(mut self, stream: impl Into<String>) -> Self {
        self.message_stream = Some(stream.into());
        self
    }

    /// Returns whether delivery will use the provider or the offline mock fallback.
    pub fn delivery_mode(&self) -> DeliveryMode {
        credential_mode(&self.server_token)
    }
}

#[async_trait]
impl MailDriver for PostmarkDriver {
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        validate_credential("Postmark server token", &self.server_token)?;
        if let Some(stream) = self.message_stream.as_deref() {
            validate_credential("Postmark message stream", stream)?;
        }
        let prepared = DeliveryPipeline::prepare(message)?;
        let message = prepared.message();
        if self.delivery_mode() == DeliveryMode::OfflineMock {
            return record_offline_delivery("postmark", message);
        }

        static HTTP_CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
        let client = HTTP_CLIENT.get_or_init(reqwest::Client::new);

        let from_addr = message.from.as_deref().unwrap_or("noreply@rullst.dev");

        let mut headers_vec = Vec::new();
        if let Some(unsub) = message.list_unsubscribe_header() {
            headers_vec.push(serde_json::json!({
                "Name": "List-Unsubscribe",
                "Value": unsub
            }));
            if message.unsubscribe_url.is_some() {
                headers_vec.push(serde_json::json!({
                    "Name": "List-Unsubscribe-Post",
                    "Value": "List-Unsubscribe=One-Click"
                }));
            }
        }

        let mut body = serde_json::json!({
            "From": from_addr,
            "To": message.to,
            "Subject": message.subject,
        });

        if let Some(ref html) = message.body_html {
            body["HtmlBody"] = serde_json::json!(html);
        }
        if let Some(ref text) = message.body_text {
            body["TextBody"] = serde_json::json!(text);
        }
        if let Some(ref stream) = self.message_stream {
            body["MessageStream"] = serde_json::json!(stream);
        }
        if !message.attachments.is_empty() {
            let attachments_json: Vec<_> = message
                .attachments
                .iter()
                .map(|att| {
                    let mut obj = serde_json::json!({
                        "Name": att.filename,
                        "Content": att.to_base64(),
                        "ContentType": att.mime_type,
                    });
                    if let Some(ref cid) = att.cid {
                        obj["ContentID"] = serde_json::json!(format!("cid:{}", cid));
                    }
                    obj
                })
                .collect();
            body["Attachments"] = serde_json::json!(attachments_json);
        }
        if !headers_vec.is_empty() {
            body["Headers"] = serde_json::json!(headers_vec);
        }

        let res = client
            .post("https://api.postmarkapp.com/email")
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("X-Postmark-Server-Token", &self.server_token)
            .json(&body)
            .send()
            .await
            .map_err(|_| MailError::transport("postmark", "request failed before response"))?;

        let status = res.status();
        if status.is_success() {
            Ok(())
        } else {
            Err(crate::error::provider_http_error("postmark", res).await)
        }
    }
}
