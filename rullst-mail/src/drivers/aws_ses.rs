// src/drivers/aws_ses.rs — AWS Simple Email Service (SES) v2 HTTP REST API driver.

use super::traits::MailDriver;
use super::{DeliveryMode, credential_mode};
use crate::drivers::mock::{record_offline_delivery, validate_credential};
use crate::error::MailError;
use crate::message::Message;
use crate::pipeline::DeliveryPipeline;
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
    /// Creates a driver after validating region and credential structure.
    pub fn try_new(
        region: impl Into<String>,
        auth_token: impl Into<String>,
    ) -> Result<Self, MailError> {
        let region = region.into();
        let auth_token = auth_token.into();
        validate_region(&region)?;
        validate_credential("AWS SES authorization token", &auth_token)?;
        Ok(Self {
            region,
            auth_token,
            endpoint_override: None,
        })
    }

    /// Creates a new `AwsSesDriver` with the specified region and authorization token.
    ///
    /// Prefer [`Self::try_new`]. Invalid configuration still fails closed in `send`.
    #[deprecated(since = "12.0.0", note = "use AwsSesDriver::try_new")]
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

    /// Fallible endpoint override which validates the URL before storing it.
    pub fn try_with_endpoint(mut self, endpoint: impl Into<String>) -> Result<Self, MailError> {
        let endpoint = endpoint.into();
        validate_endpoint(&endpoint)?;
        self.endpoint_override = Some(endpoint);
        Ok(self)
    }

    /// Returns whether delivery will use AWS SES or the offline mock fallback.
    pub fn delivery_mode(&self) -> DeliveryMode {
        credential_mode(&self.auth_token)
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
        validate_region(&self.region)?;
        validate_credential("AWS SES authorization token", &self.auth_token)?;
        if let Some(endpoint) = self.endpoint_override.as_deref() {
            validate_endpoint(endpoint)?;
        }
        let prepared = DeliveryPipeline::prepare(message)?;
        let message = prepared.message();
        if self.delivery_mode() == DeliveryMode::OfflineMock {
            return record_offline_delivery("aws_ses", message);
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

fn validate_region(region: &str) -> Result<(), MailError> {
    if region.is_empty()
        || region.len() > 64
        || !region
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err(MailError::ConfigError(
            "AWS SES region must be 1-64 ASCII letters, digits, or '-'".to_string(),
        ));
    }
    Ok(())
}

fn validate_endpoint(endpoint: &str) -> Result<(), MailError> {
    let url = reqwest::Url::parse(endpoint)
        .map_err(|_| MailError::ConfigError("AWS SES endpoint is not a valid URL".to_string()))?;
    if matches!(url.scheme(), "http" | "https") && url.host_str().is_some() {
        Ok(())
    } else {
        Err(MailError::ConfigError(
            "AWS SES endpoint must use HTTP or HTTPS and include a host".to_string(),
        ))
    }
}
