// src/drivers/sendgrid.rs — SendGrid HTTP REST API driver with RFC 8058 headers.

use super::traits::MailDriver;
use crate::error::MailError;
use crate::message::Message;
use async_trait::async_trait;

/// A SendGrid HTTP REST API driver
pub struct SendGridDriver {
    /// SendGrid API token.
    pub api_key: String,
}

#[async_trait]
impl MailDriver for SendGridDriver {
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        static HTTP_CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
        let client = HTTP_CLIENT.get_or_init(reqwest::Client::new);

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

        let mut body = serde_json::json!({
            "personalizations": personalizations,
            "from": { "email": from_addr },
            "subject": message.subject,
            "content": content
        });

        if let Some(ref send_at) = message.send_at {
            body["send_at"] = serde_json::json!(send_at.timestamp());
        }
        if !message.attachments.is_empty() {
            let attachments_json: Vec<_> = message
                .attachments
                .iter()
                .map(|att| {
                    let mut obj = serde_json::json!({
                        "content": att.to_base64(),
                        "filename": att.filename,
                        "type": att.mime_type,
                        "disposition": if att.is_inline() { "inline" } else { "attachment" }
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
