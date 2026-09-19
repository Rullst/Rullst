//! Mailtrap hosted Sending/Sandbox APIs; distinct from the local `MailTrap` harness.
use super::{DeliveryMode, credential_mode, mock, rest};
use crate::{DeliveryPipeline, MailDriver, MailError, Message};
use secrecy::{ExposeSecret, SecretString};
use serde_json::{Value, json};

/// Hosted Mailtrap API driver. Production and sandbox endpoints are selected
/// explicitly at construction; no delivery-to-sandbox fallback is performed.
pub struct MailtrapDriver {
    api_token: SecretString,
    sandbox_id: Option<u64>,
}
impl MailtrapDriver {
    /// Sends real transactional emails when supplied a real token.
    pub fn try_new(api_token: impl Into<String>) -> Result<Self, MailError> {
        let api_token = api_token.into();
        mock::validate_credential("Mailtrap API token", &api_token)?;
        Ok(Self {
            api_token: SecretString::from(api_token),
            sandbox_id: None,
        })
    }
    /// Captures mail in the explicitly selected hosted test sandbox.
    pub fn sandbox(api_token: impl Into<String>, sandbox_id: u64) -> Result<Self, MailError> {
        if sandbox_id == 0 {
            return Err(MailError::ConfigError(
                "Mailtrap sandbox ID must be positive".into(),
            ));
        }
        let mut driver = Self::try_new(api_token)?;
        driver.sandbox_id = Some(sandbox_id);
        Ok(driver)
    }
    pub fn delivery_mode(&self) -> DeliveryMode {
        credential_mode(self.api_token.expose_secret())
    }
    fn request(&self, message: &Message) -> Result<reqwest::Request, MailError> {
        let endpoint = self.sandbox_id.map_or_else(
            || "https://send.api.mailtrap.io/api/send".into(),
            |id| format!("https://sandbox.api.mailtrap.io/api/send/{id}"),
        );
        super::http::client()?
            .post(endpoint)
            .bearer_auth(self.api_token.expose_secret())
            .header("User-Agent", "rullst-mail/12.1")
            .header("Content-Type", "application/json")
            .body(rest::encode(&payload(message)?)?)
            .build()
            .map_err(|_| rest::contract())
    }
}
#[async_trait::async_trait]
impl MailDriver for MailtrapDriver {
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        let prepared = DeliveryPipeline::prepare(message)?;
        let message = prepared.message();
        if self.delivery_mode() == DeliveryMode::OfflineMock {
            return mock::record_offline_delivery("mailtrap", message);
        }
        DeliveryPipeline::require_due("Mailtrap", message)?;
        let receipt = rest::execute(self.request(message)?, "mailtrap").await?;
        validate_receipt(&receipt)
    }
}
fn payload(message: &Message) -> Result<Value, MailError> {
    let mut mail = json!({"from":{"email":rest::sender(message)?},"to":[{"email":message.to}],
        "subject":message.subject,"headers":rest::headers(message)});
    if let Some(html) = &message.body_html {
        mail["html"] = json!(html);
    }
    if let Some(text) = &message.body_text {
        mail["text"] = json!(text);
    }
    if !message.attachments.is_empty() {
        mail["attachments"] = json!(message.attachments.iter().map(|a| {
            let mut v = json!({"filename":a.filename,"type":a.mime_type,"content":a.to_base64(),
                "disposition":if a.is_inline() {"inline"} else {"attachment"}});
            if let Some(cid) = &a.cid { v["content_id"] = json!(cid); }
            v
        }).collect::<Vec<_>>());
    }
    Ok(mail)
}
fn validate_receipt(value: &Value) -> Result<(), MailError> {
    if value["success"] != true
        || value["message_ids"]
            .as_array()
            .is_none_or(|ids| ids.len() != 1 || !rest::valid_id(&ids[0]))
    {
        return Err(rest::contract());
    }
    Ok(())
}
#[cfg(test)]
#[path = "mailtrap_tests.rs"]
mod tests;
