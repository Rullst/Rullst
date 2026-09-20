//! Mailjet Send API v3.1, with explicit provider sandbox validation.
use super::{DeliveryMode, credential_mode, mock, rest};
use crate::{DeliveryPipeline, MailDriver, MailError, Message};
use secrecy::{ExposeSecret, SecretString};
use serde_json::{Value, json};

/// Native Mailjet driver. Open/click tracking is explicitly disabled per message.
/// Both credentials must select the same real/offline mode; partial credentials
/// never silently downgrade to an offline success.
pub struct MailjetDriver {
    api_key: SecretString,
    secret_key: SecretString,
    sandbox: bool,
}

impl MailjetDriver {
    pub fn try_new(
        api_key: impl Into<String>,
        secret_key: impl Into<String>,
    ) -> Result<Self, MailError> {
        let api_key = api_key.into();
        let secret_key = secret_key.into();
        mock::validate_credential("Mailjet API key", &api_key)?;
        mock::validate_credential("Mailjet secret key", &secret_key)?;
        if credential_mode(&api_key) != credential_mode(&secret_key) {
            return Err(MailError::ConfigError(
                "Mailjet requires a complete credential pair".into(),
            ));
        }
        Ok(Self {
            api_key: SecretString::from(api_key),
            secret_key: SecretString::from(secret_key),
            sandbox: false,
        })
    }
    /// Remote validation with real credentials, without delivering mail.
    pub fn with_sandbox(mut self) -> Self {
        self.sandbox = true;
        self
    }
    pub fn delivery_mode(&self) -> DeliveryMode {
        credential_mode(self.api_key.expose_secret())
    }

    fn request(&self, message: &Message) -> Result<reqwest::Request, MailError> {
        super::http::client()?
            .post("https://api.mailjet.com/v3.1/send")
            .basic_auth(
                self.api_key.expose_secret(),
                Some(self.secret_key.expose_secret()),
            )
            .header("Content-Type", "application/json")
            .body(rest::encode(&payload(message, self.sandbox)?)?)
            .build()
            .map_err(|_| rest::contract())
    }
}

#[async_trait::async_trait]
impl MailDriver for MailjetDriver {
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        let prepared = DeliveryPipeline::prepare(message)?;
        let message = prepared.message();
        if self.delivery_mode() == DeliveryMode::OfflineMock {
            return mock::record_offline_delivery("mailjet", message);
        }
        DeliveryPipeline::require_due("Mailjet", message)?;
        let receipt = rest::execute(self.request(message)?, "mailjet").await?;
        validate_receipt(&receipt, self.sandbox, &message.to)
    }
}

fn payload(message: &Message, sandbox: bool) -> Result<Value, MailError> {
    let mut mail = json!({"From":{"Email":rest::sender(message)?},"To":[{"Email":message.to}],
        "Subject":message.subject,"TrackOpens":"disabled","TrackClicks":"disabled","Headers":rest::headers(message)});
    if let Some(html) = &message.body_html {
        mail["HTMLPart"] = json!(html);
    }
    if let Some(text) = &message.body_text {
        mail["TextPart"] = json!(text);
    }
    let mut attached = Vec::new();
    let mut inline = Vec::new();
    for item in &message.attachments {
        let mut value = json!({"ContentType":item.mime_type,"Filename":item.filename,"Base64Content":item.to_base64()});
        if let Some(cid) = &item.cid {
            value["ContentID"] = json!(cid);
            inline.push(value);
        } else {
            attached.push(value);
        }
    }
    if !attached.is_empty() {
        mail["Attachments"] = json!(attached);
    }
    if !inline.is_empty() {
        mail["InlinedAttachments"] = json!(inline);
    }
    Ok(json!({"Messages":[mail],"SandboxMode":sandbox}))
}

fn validate_receipt(value: &Value, sandbox: bool, recipient: &str) -> Result<(), MailError> {
    let messages = value["Messages"]
        .as_array()
        .filter(|v| v.len() == 1)
        .ok_or_else(rest::contract)?;
    let message = &messages[0];
    if message["Status"] != "success" {
        return Err(rest::contract());
    }
    let recipients = message["To"]
        .as_array()
        .filter(|v| v.len() == 1)
        .ok_or_else(rest::contract)?;
    if recipients[0]["Email"].as_str() != Some(recipient)
        || (!sandbox
            && (recipients[0]["MessageID"].as_u64().is_none_or(|id| id == 0)
                || !rest::valid_id(&recipients[0]["MessageUUID"])))
    {
        return Err(rest::contract());
    }
    Ok(())
}

#[cfg(test)]
#[path = "mailjet_tests.rs"]
mod tests;
