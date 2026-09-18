//! SendPulse SMTP REST API with a static API key (Bearer authentication).
use super::{DeliveryMode, credential_mode, mock, rest};
use crate::{DeliveryPipeline, MailDriver, MailError, Message};
use base64::{Engine, engine::general_purpose::STANDARD};
use secrecy::{ExposeSecret, SecretString};
use serde_json::{Value, json};

/// Native transactional SendPulse adapter. The account's SMTP service and
/// sender must be activated. Disable tracking in the SendPulse account settings.
/// Uses a static API key, not an OAuth client ID/secret pair.
pub struct SendPulseDriver {
    api_key: SecretString,
}

impl SendPulseDriver {
    pub fn try_new(api_key: impl Into<String>) -> Result<Self, MailError> {
        let api_key = api_key.into();
        mock::validate_credential("SendPulse API key", &api_key)?;
        Ok(Self {
            api_key: SecretString::from(api_key),
        })
    }
    pub fn delivery_mode(&self) -> DeliveryMode {
        credential_mode(self.api_key.expose_secret())
    }

    fn request(&self, message: &Message) -> Result<reqwest::Request, MailError> {
        let body = payload(message)?;
        super::http::client()?
            .post("https://api.sendpulse.com/smtp/emails")
            .bearer_auth(self.api_key.expose_secret())
            .header("Content-Type", "application/json")
            .body(rest::encode(&body)?)
            .build()
            .map_err(|_| rest::contract())
    }
}

#[async_trait::async_trait]
impl MailDriver for SendPulseDriver {
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        let prepared = DeliveryPipeline::prepare(message)?;
        let message = prepared.message();
        if self.delivery_mode() == DeliveryMode::OfflineMock {
            return mock::record_offline_delivery("sendpulse", message);
        }
        DeliveryPipeline::require_due("SendPulse", message)?;
        let receipt = rest::execute(self.request(message)?, "sendpulse").await?;
        validate_receipt(&receipt)
    }
}

fn payload(message: &Message) -> Result<Value, MailError> {
    // This documented API has no reviewed inline-CID or custom header contract.
    // Never silently drop security/unsubscribe semantics.
    if message.attachments.iter().any(|a| a.is_inline())
        || message.list_unsubscribe_header().is_some()
    {
        return Err(MailError::ValidationError("SendPulse REST does not support this driver's inline-CID or unsubscribe-header contract; use SMTP".into()));
    }
    let mut email = json!({"subject":message.subject,"from":{"email":rest::sender(message)?},
        "to":[{"email":message.to}],"auto_plain_text":false});
    if let Some(html) = &message.body_html {
        email["html"] = json!(STANDARD.encode(html));
    }
    if let Some(text) = &message.body_text {
        email["text"] = json!(text);
    }
    if !message.attachments.is_empty() {
        let mut files = serde_json::Map::new();
        for item in &message.attachments {
            if files
                .insert(item.filename.clone(), json!(item.to_base64()))
                .is_some()
            {
                return Err(MailError::ValidationError(
                    "SendPulse attachment names must be unique".into(),
                ));
            }
        }
        email["attachments_binary"] = Value::Object(files);
    }
    Ok(json!({"email":email}))
}

fn validate_receipt(value: &Value) -> Result<(), MailError> {
    if value["result"] != true || !rest::valid_id(&value["id"]) {
        return Err(rest::contract());
    }
    Ok(())
}

#[cfg(test)]
#[path = "sendpulse_tests.rs"]
mod tests;
