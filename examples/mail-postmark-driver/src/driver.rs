//! `MailDriver` for Postmark's Email API.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderValue, RETRY_AFTER};
use rullst_mail::{
    Attachment, DeliveryMode, DeliveryPipeline, MailDriver, MailError, Message, credential_mode,
};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Stable low-cardinality label used in errors, telemetry and suppression
/// events.
pub const PROVIDER: &str = "postmark";
/// Postmark's single-message endpoint.
const ENDPOINT: &str = "https://api.postmarkapp.com/email";
/// Postmark answers 413 to an Email API request over 10 MB.
const MAX_REQUEST_BYTES: usize = 10_000_000;
/// Largest delivery receipt read from Postmark.
const MAX_RECEIPT_BYTES: usize = 16 * 1024;
/// Longest server token accepted.
const MAX_TOKEN_BYTES: usize = 4096;

/// Messages the offline mode captured instead of sending.
pub type OfflineOutbox = Arc<Mutex<Vec<Message>>>;

/// Transport for Postmark's `POST /email`.
pub struct PostmarkDriver {
    /// `X-Postmark-Server-Token`, marked sensitive so `Debug` hides it.
    token: HeaderValue,
    mode: DeliveryMode,
    endpoint: String,
    client: reqwest::Client,
    outbox: OfflineOutbox,
}

impl PostmarkDriver {
    /// Creates the transport with a Postmark server token. An empty or
    /// `mock_*` token selects the offline mode.
    pub fn try_new(server_token: impl Into<String>) -> Result<Self, MailError> {
        let server_token = server_token.into();
        let malformed = || MailError::ConfigError("Postmark server token is malformed".into());
        if server_token.len() > MAX_TOKEN_BYTES {
            return Err(malformed());
        }
        let mut token = HeaderValue::from_str(&server_token).map_err(|_| malformed())?;
        token.set_sensitive(true);
        // No redirects (they would forward the token and the message), and
        // deadlines for connecting and for the whole exchange.
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|_| MailError::ConfigError("cannot build the Postmark client".into()))?;
        Ok(Self {
            token,
            mode: credential_mode(&server_token),
            endpoint: ENDPOINT.into(),
            client,
            outbox: OfflineOutbox::default(),
        })
    }

    /// Uses another endpoint: HTTPS, or plain HTTP to a loopback test fixture.
    pub fn try_with_endpoint(mut self, endpoint: impl Into<String>) -> Result<Self, MailError> {
        let endpoint = endpoint.into();
        if !endpoint.starts_with("https://") && !endpoint.starts_with("http://127.0.0.1:") {
            return Err(MailError::ConfigError(
                "Postmark endpoint must use HTTPS".into(),
            ));
        }
        self.endpoint = endpoint;
        Ok(self)
    }

    /// Whether the token selects Postmark or the offline mode.
    pub fn delivery_mode(&self) -> DeliveryMode {
        self.mode
    }

    /// The offline mode's captured messages, as they would have been sent.
    pub fn offline_outbox(&self) -> OfflineOutbox {
        Arc::clone(&self.outbox)
    }
}

/// The JSON request body for a pipeline-validated message. What Postmark
/// cannot express is an error, never silently dropped.
pub fn request_body(message: &Message) -> Result<Vec<u8>, MailError> {
    let body = serde_json::to_vec(&payload(message, chrono::Utc::now())?)
        .map_err(|_| MailError::DriverError("cannot encode the Postmark request".into()))?;
    if body.len() > MAX_REQUEST_BYTES {
        return Err(MailError::ValidationError(
            "Postmark accepts at most 10 MB per message, attachments included".into(),
        ));
    }
    Ok(body)
}

/// The request object; a `send_at` after `now` is a schedule Postmark
/// cannot keep.
fn payload(message: &Message, now: DateTime<Utc>) -> Result<Value, MailError> {
    let from = message.from.as_deref().ok_or_else(|| {
        MailError::ConfigError(
            "Postmark requires a confirmed sender signature: set `from`, or MAIL_FROM for the \
             Mail facade"
                .into(),
        )
    })?;
    let text = message.body_text.as_deref().filter(|body| !body.is_empty());
    let html = message.body_html.as_deref().filter(|body| !body.is_empty());
    if text.is_none() && html.is_none() {
        return Err(MailError::ValidationError(
            "Postmark requires a text or HTML body".into(),
        ));
    }
    if message.send_at.is_some_and(|send_at| send_at > now) {
        return Err(MailError::ConfigError(
            "Postmark cannot schedule mail; use Mail::enqueue with a durable queue".into(),
        ));
    }
    let mut body = json!({ "From": from, "To": message.to, "Subject": message.subject });
    if let Some(text) = text {
        body["TextBody"] = json!(text);
    }
    if let Some(html) = html {
        body["HtmlBody"] = json!(html);
    }
    if let Some(reply_to) = &message.reply_to {
        body["ReplyTo"] = json!(reply_to);
    }
    if let Some(value) = message.list_unsubscribe_header() {
        let mut headers = vec![json!({ "Name": "List-Unsubscribe", "Value": value })];
        if one_click(message) {
            headers.push(json!({
                "Name": "List-Unsubscribe-Post",
                "Value": "List-Unsubscribe=One-Click",
            }));
        }
        body["Headers"] = Value::Array(headers);
    }
    if !message.attachments.is_empty() {
        body["Attachments"] = message.attachments.iter().map(attachment).collect();
    }
    Ok(body)
}

/// One attachment; an inline image keeps its CID as `cid:<id>`, the form
/// Postmark documents.
fn attachment(item: &Attachment) -> Value {
    let mut value = json!({
        "Name": item.filename,
        "Content": item.to_base64(),
        "ContentType": item.mime_type,
    });
    if let Some(cid) = &item.cid {
        value["ContentID"] = json!(format!("cid:{cid}"));
    }
    value
}

/// RFC 8058 one-click unsubscription requires an HTTPS URI.
fn one_click(message: &Message) -> bool {
    message
        .unsubscribe_url
        .as_deref()
        .and_then(|url| url.get(..8))
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https://"))
}

/// Whether a 2xx receipt reports an accepted message: `ErrorCode` 0 and a
/// bounded `MessageID` of letters, digits and hyphens.
pub fn accepted(receipt: &Value) -> bool {
    receipt["ErrorCode"] == 0
        && receipt["MessageID"].as_str().is_some_and(|id| {
            !id.is_empty()
                && id.len() <= 128
                && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
}

/// Reads at most `MAX_RECEIPT_BYTES` and requires an accepted message.
async fn receipt(mut response: reqwest::Response) -> Result<(), MailError> {
    let invalid = || MailError::SendError("Postmark returned an invalid receipt".into());
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| MailError::transport(PROVIDER, "receipt read failed"))?
    {
        if body.len() + chunk.len() > MAX_RECEIPT_BYTES {
            return Err(invalid());
        }
        body.extend_from_slice(&chunk);
    }
    let receipt: Value = serde_json::from_slice(&body).map_err(|_| invalid())?;
    if accepted(&receipt) {
        Ok(())
    } else {
        Err(invalid())
    }
}

/// A delta-seconds `Retry-After`, capped at one day.
fn retry_after(response: &reqwest::Response) -> Option<Duration> {
    response
        .headers()
        .get(RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(|seconds| Duration::from_secs(seconds.min(86_400)))
}

#[async_trait]
impl MailDriver for PostmarkDriver {
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        // The shared pre-flight checks: header injection, recipients,
        // outbound secrets, links and attachment bounds.
        let prepared = DeliveryPipeline::prepare(message)?;
        let message = prepared.message();
        // Built before the offline check, so the offline mode refuses what
        // Postmark would refuse.
        let body = request_body(message)?;
        if self.mode == DeliveryMode::OfflineMock {
            self.outbox
                .lock()
                .map_err(|_| MailError::DriverError("Postmark outbox is unavailable".into()))?
                .push(message.clone());
            return Ok(());
        }
        let response = self
            .client
            .post(&self.endpoint)
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json")
            .header("X-Postmark-Server-Token", self.token.clone())
            .body(body)
            .send()
            .await
            .map_err(|_| MailError::transport(PROVIDER, "request failed before a response"))?;
        if response.status().is_success() {
            return receipt(response).await;
        }
        // Postmark's error bodies (`ErrorCode`, `Message`) can echo
        // addresses: keep the status and the retry delay only. 429 is
        // `RateLimited`, 5xx transient, 401 and 422 (406 is an inactive,
        // suppressed recipient) permanent.
        Err(MailError::from_provider_response(
            PROVIDER,
            response.status().as_u16(),
            "provider response body omitted",
            retry_after(&response),
        ))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "driver_tests.rs"]
mod tests;
