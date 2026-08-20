// src/drivers/postmark.rs — Postmark HTTP REST API driver with RFC 8058 headers.

use crate::drivers::MailDriver;
use crate::error::MailError;
use crate::message::Message;
use async_trait::async_trait;

/// A Postmark HTTP REST API driver.
pub struct PostmarkDriver {
    /// Postmark server API token (`X-Postmark-Server-Token`).
    pub server_token: String,
    /// Optional custom message stream (defaults to `"outbound"` in Postmark).
    pub message_stream: Option<String>,
}

impl PostmarkDriver {
    /// Creates a new `PostmarkDriver` with the given server API token.
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
}

#[async_trait]
impl MailDriver for PostmarkDriver {
    async fn send(&self, message: &Message) -> Result<(), MailError> {
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
            .map_err(|e| MailError::SendError(e.to_string()))?;

        let status = res.status();
        if status.is_success() {
            Ok(())
        } else {
            let text = res.text().await.unwrap_or_default();
            Err(MailError::SendError(format!(
                "Postmark API error (status {}): {}",
                status, text
            )))
        }
    }
}
