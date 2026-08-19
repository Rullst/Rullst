// src/drivers/resend.rs — Resend HTTP REST API driver with RFC 8058 headers.

use super::{MailDriver, MailError};
use crate::message::Message;
use async_trait::async_trait;

/// A Resend HTTP REST API driver
pub struct ResendDriver {
    /// Resend API token.
    pub api_key: String,
}

#[async_trait]
impl MailDriver for ResendDriver {
    async fn send(&self, message: &Message) -> Result<(), MailError> {
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
