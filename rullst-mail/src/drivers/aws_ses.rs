// src/drivers/aws_ses.rs — AWS Simple Email Service (SES) v2 HTTP REST API driver.

use crate::drivers::MailDriver;
use crate::error::MailError;
use crate::message::Message;
use async_trait::async_trait;

/// An AWS SES v2 HTTP REST API driver.
pub struct AwsSesDriver {
    /// AWS Region (e.g. `"us-east-1"`, `"sa-east-1"`).
    pub region: String,
    /// AWS Bearer token or authorization secret.
    pub auth_token: String,
    /// Optional custom endpoint URL override (useful for LocalStack, mock servers, or VPC endpoints).
    pub endpoint_override: Option<String>,
}

impl AwsSesDriver {
    /// Creates a new `AwsSesDriver` with the specified region and authorization token.
    pub fn new(region: impl Into<String>, auth_token: impl Into<String>) -> Self {
        Self {
            region: region.into(),
            auth_token: auth_token.into(),
            endpoint_override: None,
        }
    }

    /// Sets a custom endpoint URL (e.g. for testing with LocalStack or an API proxy).
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint_override = Some(endpoint.into());
        self
    }

    /// Resolves the active endpoint URL.
    pub fn endpoint(&self) -> String {
        if let Some(ref ep) = self.endpoint_override {
            ep.clone()
        } else {
            format!(
                "https://email.{}.amazonaws.com/v2/email/outbound-emails",
                self.region
            )
        }
    }
}

#[async_trait]
impl MailDriver for AwsSesDriver {
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

        let mut body_obj = serde_json::Map::new();
        if let Some(ref html) = message.body_html {
            body_obj.insert(
                "Html".to_string(),
                serde_json::json!({ "Data": html, "Charset": "UTF-8" }),
            );
        }
        if let Some(ref text) = message.body_text {
            body_obj.insert(
                "Text".to_string(),
                serde_json::json!({ "Data": text, "Charset": "UTF-8" }),
            );
        }

        let mut simple_obj = serde_json::json!({
            "Subject": {
                "Data": message.subject,
                "Charset": "UTF-8"
            },
            "Body": body_obj
        });

        if !headers_vec.is_empty() {
            simple_obj["Headers"] = serde_json::json!(headers_vec);
        }

        let payload = serde_json::json!({
            "FromEmailAddress": from_addr,
            "Destination": {
                "ToAddresses": [message.to]
            },
            "Content": {
                "Simple": simple_obj
            }
        });

        let url = self.endpoint();
        let res = client
            .post(&url)
            .header("Content-Type", "application/json")
            .bearer_auth(&self.auth_token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| MailError::SendError(e.to_string()))?;

        let status = res.status();
        if status.is_success() {
            Ok(())
        } else {
            let text = res.text().await.unwrap_or_default();
            Err(MailError::SendError(format!(
                "AWS SES API error (status {}): {}",
                status, text
            )))
        }
    }
}
