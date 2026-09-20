//! Azure Communication Services Email REST 2023-03-31 with Entra credentials.
mod credential;
pub use credential::{
    AzureMailAccessToken, AzureMailCredential, AzureManagedIdentity, StaticAzureMailCredential,
};

use crate::{DeliveryPipeline, MailDriver, MailError, Message};
use secrecy::ExposeSecret;
use serde_json::{Value, json};

/// Native ACS driver. Success means Azure completed its send operation, not that
/// a recipient read or received the message. DNS, verified domains, IAM, quotas
/// and signed Event Grid delivery feedback remain deployment responsibilities.
pub struct AzureCommunicationDriver<C> {
    endpoint: Option<reqwest::Url>,
    credential: C,
}

impl<C: AzureMailCredential> AzureCommunicationDriver<C> {
    /// Uses a public ACS HTTPS resource endpoint. Empty/`mock_*` endpoints are offline.
    pub fn new(endpoint: impl Into<String>, credential: C) -> Result<Self, MailError> {
        let endpoint = endpoint.into();
        let endpoint = if endpoint.is_empty() || endpoint.starts_with("mock_") {
            None
        } else {
            let url = reqwest::Url::parse(&endpoint).map_err(|_| config())?;
            if url.scheme() != "https"
                || !url
                    .host_str()
                    .is_some_and(|host| host.ends_with(".communication.azure.com"))
                || !url.username().is_empty()
                || url.password().is_some()
                || url.port().is_some()
                || url.path() != "/"
                || url.query().is_some()
                || url.fragment().is_some()
            {
                return Err(config());
            }
            Some(url)
        };
        Ok(Self {
            endpoint,
            credential,
        })
    }
}

#[async_trait::async_trait]
impl<C: AzureMailCredential> MailDriver for AzureCommunicationDriver<C> {
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        let message = DeliveryPipeline::prepare(message)?.into_message();
        let Some(endpoint) = &self.endpoint else {
            return super::mock::record_offline_delivery("azure-acs", &message);
        };
        let credential = self.credential.access_token().await?;
        let token = credential.value.expose_secret();
        if super::credential_mode(token) == super::DeliveryMode::OfflineMock {
            return super::mock::record_offline_delivery("azure-acs", &message);
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| config())?
            .as_secs();
        if credential.expires_at <= now.saturating_add(30) {
            return Err(config());
        }
        DeliveryPipeline::require_due("Azure Communication Services", &message)?;
        let body = payload(&message)?;
        let url = endpoint
            .join("emails:send?api-version=2023-03-31")
            .map_err(|_| config())?;
        let response = super::http::client()?
            .post(url)
            .bearer_auth(token)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(|_| MailError::transport("azure-acs", "send request failed"))?;
        if response.status() != reqwest::StatusCode::ACCEPTED {
            return Err(crate::error::provider_http_error("azure-acs", response).await);
        }
        let operation = response
            .headers()
            .get("Operation-Location")
            .and_then(|value| value.to_str().ok())
            .ok_or_else(config)?;
        let operation = reqwest::Url::parse(operation).map_err(|_| config())?;
        let result = read_json(response).await?;
        let id = result["id"]
            .as_str()
            .filter(|id| valid_uuid(id))
            .ok_or_else(config)?;
        if operation.origin() != endpoint.origin()
            || operation.path().trim_start_matches('/') != format!("emails/operations/{id}")
            || operation.username() != ""
            || operation.password().is_some()
            || operation.fragment().is_some()
            || operation.query() != Some("api-version=2023-03-31")
        {
            return Err(config());
        }
        // Bounded polling. Do not follow an arbitrary operation URL with a bearer token.
        for _ in 0..6 {
            let response = super::http::client()?
                .get(operation.clone())
                .bearer_auth(token)
                .timeout(std::time::Duration::from_secs(3))
                .send()
                .await
                .map_err(|_| MailError::transport("azure-acs", "operation status unavailable"))?;
            if !response.status().is_success() {
                return Err(crate::error::provider_http_error("azure-acs", response).await);
            }
            let status = read_json(response).await?;
            if status["id"].as_str() != Some(id) {
                return Err(config());
            }
            match status["status"].as_str() {
                Some("Succeeded") => return Ok(()),
                Some("Running" | "NotStarted") => {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await
                }
                Some("Failed" | "Canceled") => {
                    return Err(MailError::SendError("Azure email operation failed".into()));
                }
                _ => return Err(config()),
            }
        }
        Err(MailError::transport(
            "azure-acs",
            "send outcome requires operation reconciliation",
        ))
    }
}

fn payload(message: &Message) -> Result<Vec<u8>, MailError> {
    let from = message
        .from
        .as_deref()
        .ok_or_else(|| MailError::ConfigError("Azure email requires a verified sender".into()))?;
    if message
        .attachments
        .iter()
        .any(|attachment| attachment.is_inline())
    {
        return Err(MailError::ValidationError(
            "ACS 2023-03-31 does not support this driver's inline-CID contract".into(),
        ));
    }
    let mut value = json!({"senderAddress":from,"recipients":{"to":[{"address":message.to}]},
        "content":{"subject":message.subject},"userEngagementTrackingDisabled":true});
    if let Some(html) = &message.body_html {
        value["content"]["html"] = json!(html);
    }
    if let Some(text) = &message.body_text {
        value["content"]["plainText"] = json!(text);
    }
    if !message.attachments.is_empty() {
        value["attachments"] = json!(message.attachments.iter().map(|item| json!({"name":item.filename,"contentType":item.mime_type,"contentInBase64":item.to_base64()})).collect::<Vec<_>>());
    }
    if let Some(header) = message.list_unsubscribe_header() {
        value["headers"] = json!({"List-Unsubscribe":header});
        if message.unsubscribe_url.is_some() {
            value["headers"]["List-Unsubscribe-Post"] = json!("List-Unsubscribe=One-Click");
        }
    }
    let bytes = serde_json::to_vec(&value).map_err(|_| config())?;
    if bytes.len() > 10_000_000 {
        return Err(MailError::ValidationError(
            "Azure email request exceeds 10 MB".into(),
        ));
    }
    Ok(bytes)
}

pub(super) async fn read_json(mut response: reqwest::Response) -> Result<Value, MailError> {
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| MailError::transport("azure-acs", "response read failed"))?
    {
        if bytes.len() + chunk.len() > 65536 {
            return Err(config());
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| config())
}

fn valid_uuid(id: &str) -> bool {
    id.len() == 36
        && id.bytes().enumerate().all(|(index, byte)| {
            if [8, 13, 18, 23].contains(&index) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}
fn config() -> MailError {
    MailError::ConfigError("invalid Azure mail endpoint or response contract".into())
}

#[cfg(test)]
mod tests;
