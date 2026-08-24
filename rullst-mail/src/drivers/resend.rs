// src/drivers/resend.rs — Resend HTTP REST API driver with RFC 8058 headers.

use super::traits::MailDriver;
use super::{DeliveryMode, credential_mode};
use crate::drivers::mock::{record_offline_delivery, validate_credential};
use crate::error::MailError;
use crate::message::Message;
use crate::pipeline::DeliveryPipeline;
use async_trait::async_trait;

/// A Resend HTTP REST API driver
pub struct ResendDriver {
    /// Resend API token.
    pub api_key: String,
}

impl ResendDriver {
    /// Creates a driver after validating the credential's structural safety.
    pub fn try_new(api_key: impl Into<String>) -> Result<Self, MailError> {
        let api_key = api_key.into();
        validate_credential("Resend API key", &api_key)?;
        Ok(Self { api_key })
    }

    /// Creates a driver while preserving the legacy infallible API.
    ///
    /// Prefer [`Self::try_new`]. Invalid configuration still fails closed in `send`.
    #[deprecated(since = "12.0.0", note = "use ResendDriver::try_new")]
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
        }
    }

    /// Returns whether delivery will use the provider or the offline mock fallback.
    pub fn delivery_mode(&self) -> DeliveryMode {
        credential_mode(&self.api_key)
    }
}

#[async_trait]
impl MailDriver for ResendDriver {
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        validate_credential("Resend API key", &self.api_key)?;
        let prepared = DeliveryPipeline::prepare(message)?;
        let message = prepared.message();
        if self.delivery_mode() == DeliveryMode::OfflineMock {
            return record_offline_delivery("resend", message);
        }

        static HTTP_CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
        let client = HTTP_CLIENT.get_or_init(reqwest::Client::new);

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
        if let Some(ref send_at) = message.send_at {
            body["scheduled_at"] = serde_json::json!(send_at.to_rfc3339());
        }
        if !message.attachments.is_empty() {
            let attachments_json: Vec<_> = message
                .attachments
                .iter()
                .map(|att| {
                    let mut obj = serde_json::json!({
                        "filename": att.filename,
                        "content": att.to_base64(),
                    });
                    if let Some(ref cid) = att.cid {
                        obj["content_id"] = serde_json::json!(cid);
                    }
                    obj
                })
                .collect();
            body["attachments"] = serde_json::json!(attachments_json);
        }
        if let Some(unsub) = message.list_unsubscribe_header() {
            let mut headers_obj = serde_json::json!({
                "List-Unsubscribe": unsub
            });
            if message.unsubscribe_url.is_some() {
                headers_obj["List-Unsubscribe-Post"] =
                    serde_json::json!("List-Unsubscribe=One-Click");
            }
            body["headers"] = headers_obj;
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
