//! Writing your own mail transport: a complete example that runs offline.
//!
//! `AcmeMail` is a fictitious HTTP email API. The transport implements
//! `MailDriver`, runs the shared pre-flight pipeline, rejects what the API
//! cannot express, captures mail offline for empty or `mock_*` keys and maps
//! HTTP failures to typed errors. Run it with:
//!
//! ```text
//! cargo run -p rullst-mail --example custom_transport
//! ```
//!
//! The walkthrough is `docs/src/mail-custom-transport.md`.

use async_trait::async_trait;
use rullst_mail::{
    DeliveryMode, DeliveryPipeline, FailoverDriver, InMemorySuppressionStore, Mail, MailDriver,
    MailError, MailFailureClass, MemoryDriver, Message, MutableSuppressionStore, SuppressionEvent,
    SuppressionGuard, SuppressionReason, TenantMailResolver, credential_mode,
};
use secrecy::{ExposeSecret, SecretString};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// ANCHOR: transport
/// Stable low-cardinality label used in errors and telemetry.
const PROVIDER: &str = "acmemail";
/// Largest delivery receipt read from AcmeMail.
const MAX_RECEIPT_BYTES: usize = 16 * 1024;

/// Messages the offline fixture captured instead of sending.
pub type OfflineOutbox = Arc<Mutex<Vec<Message>>>;

/// Transport for the fictitious AcmeMail HTTP API.
pub struct AcmeMailDriver {
    api_key: SecretString,
    endpoint: String,
    client: reqwest::Client,
    outbox: OfflineOutbox,
}

impl AcmeMailDriver {
    /// Creates the transport. Empty or `mock_*` keys select the offline fixture.
    pub fn try_new(api_key: impl Into<String>) -> Result<Self, MailError> {
        let api_key = api_key.into();
        if api_key.contains(['\r', '\n']) || api_key.len() > 4096 {
            return Err(MailError::ConfigError(
                "AcmeMail API key is malformed".into(),
            ));
        }
        // No redirects (they would forward the key and the message), and
        // deadlines for connecting and for the whole exchange.
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|_| MailError::ConfigError("cannot build the AcmeMail client".into()))?;
        Ok(Self {
            api_key: SecretString::from(api_key),
            endpoint: "https://api.acmemail.invalid/v1/messages".into(),
            client,
            outbox: OfflineOutbox::default(),
        })
    }

    /// Uses another endpoint: HTTPS, or plain HTTP to a loopback test fixture.
    pub fn try_with_endpoint(mut self, endpoint: impl Into<String>) -> Result<Self, MailError> {
        let endpoint = endpoint.into();
        let loopback = endpoint.starts_with("http://127.0.0.1:");
        if !endpoint.starts_with("https://") && !loopback {
            return Err(MailError::ConfigError(
                "AcmeMail endpoint must use HTTPS".into(),
            ));
        }
        self.endpoint = endpoint;
        Ok(self)
    }

    /// Whether the key selects the real API or the offline fixture.
    pub fn delivery_mode(&self) -> DeliveryMode {
        credential_mode(self.api_key.expose_secret())
    }

    /// The offline fixture's captured messages, shared with every clone.
    pub fn offline_outbox(&self) -> OfflineOutbox {
        Arc::clone(&self.outbox)
    }

    /// The API request body. Content the API cannot express is an error,
    /// never silently dropped.
    fn payload(message: &Message) -> Result<Value, MailError> {
        let from = message.from.as_deref().ok_or_else(|| {
            MailError::ConfigError(
                "AcmeMail requires a verified sender: set `from`, or MAIL_FROM for the Mail facade"
                    .into(),
            )
        })?;
        if message
            .send_at
            .is_some_and(|send_at| send_at > chrono::Utc::now())
        {
            return Err(MailError::ConfigError(
                "AcmeMail cannot schedule mail; use Mail::enqueue with a durable queue".into(),
            ));
        }
        if message.attachments.iter().any(|item| item.is_inline()) {
            return Err(MailError::ValidationError(
                "AcmeMail has no inline images; use a transport that supports CIDs".into(),
            ));
        }
        let mut headers = serde_json::Map::new();
        if let Some(value) = message.list_unsubscribe_header() {
            headers.insert("List-Unsubscribe".into(), Value::String(value));
            // RFC 8058 one-click unsubscription requires an HTTPS URI.
            if message
                .unsubscribe_url
                .as_deref()
                .is_some_and(|url| url.starts_with("https://"))
            {
                headers.insert(
                    "List-Unsubscribe-Post".into(),
                    Value::String("List-Unsubscribe=One-Click".into()),
                );
            }
        }
        let attachments: Vec<Value> = message
            .attachments
            .iter()
            .map(|item| {
                json!({
                    "filename": item.filename,
                    "content_type": item.mime_type,
                    "content": item.to_base64(),
                })
            })
            .collect();
        Ok(json!({
            "from": from,
            "to": message.to,
            "subject": message.subject,
            "text": message.body_text,
            "html": message.body_html,
            "headers": headers,
            "attachments": attachments,
        }))
    }

    /// Reads at most `MAX_RECEIPT_BYTES` and requires a message ID.
    async fn receipt(mut response: reqwest::Response) -> Result<(), MailError> {
        let invalid = || MailError::SendError("AcmeMail returned an invalid receipt".into());
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
        match receipt.get("id").and_then(Value::as_str) {
            Some(id) if !id.is_empty() && id.len() <= 128 => Ok(()),
            _ => Err(invalid()),
        }
    }
}

#[async_trait]
impl MailDriver for AcmeMailDriver {
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        // The shared pre-flight checks: header injection, recipients,
        // outbound secrets, links and attachment bounds.
        let prepared = DeliveryPipeline::prepare(message)?;
        let message = prepared.message();
        let body = Self::payload(message)?;
        if self.delivery_mode() == DeliveryMode::OfflineMock {
            self.outbox
                .lock()
                .map_err(|_| MailError::DriverError("AcmeMail outbox is unavailable".into()))?
                .push(message.clone());
            return Ok(());
        }
        let response = self
            .client
            .post(&self.endpoint)
            .bearer_auth(self.api_key.expose_secret())
            .json(&body)
            .send()
            .await
            .map_err(|_| MailError::transport(PROVIDER, "request failed before a response"))?;
        if response.status().is_success() {
            return Self::receipt(response).await;
        }
        let retry_after = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
            .map(|seconds| Duration::from_secs(seconds.min(86_400)));
        // Error bodies can echo addresses and content: keep only the status
        // and the retry delay. 429 becomes `RateLimited`, 5xx is transient.
        Err(MailError::from_provider_response(
            PROVIDER,
            response.status().as_u16(),
            "provider response body omitted",
            retry_after,
        ))
    }
}
// ANCHOR_END: transport

fn welcome(to: &str) -> Message {
    Message::new()
        .to(to)
        .from("Acme <hello@acme.example>")
        .subject("Welcome to Acme")
        .text("Thanks for signing up.")
}

/// Serves one canned HTTP response per connection on a loopback port.
async fn loopback_fixture(responses: Vec<&'static str>) -> std::io::Result<String> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let endpoint = format!("http://{}/v1/messages", listener.local_addr()?);
    tokio::spawn(async move {
        for response in responses {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            // Read the whole request before answering.
            let mut request = Vec::new();
            let mut buffer = [0_u8; 4096];
            while let Ok(read) = socket.read(&mut buffer).await {
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
                let text = String::from_utf8_lossy(&request).to_ascii_lowercase();
                if let Some((head, body)) = text.split_once("\r\n\r\n") {
                    let length = head
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length:"))
                        .and_then(|value| value.trim().parse::<usize>().ok())
                        .unwrap_or(0);
                    if body.len() >= length {
                        break;
                    }
                }
            }
            let _ = socket.write_all(response.as_bytes()).await;
        }
    });
    Ok(endpoint)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. A `mock_*` key captures mail offline; it never reaches the network.
    let offline = AcmeMailDriver::try_new("mock_acme")?;
    assert_eq!(offline.delivery_mode(), DeliveryMode::OfflineMock);
    let outbox = offline.offline_outbox();
    offline.send(&welcome("ada@example.com")).await?;

    // 2. What the API cannot express fails instead of being dropped.
    let anonymous = Message::new()
        .to("ada@example.com")
        .subject("No sender")
        .text("body");
    assert!(matches!(
        offline.send(&anonymous).await,
        Err(MailError::ConfigError(_))
    ));
    let inline =
        welcome("ada@example.com").attach_cid("logo", "logo.png", vec![1, 2, 3], "image/png");
    assert!(matches!(
        offline.send(&inline.html("<img src=\"cid:logo\">")).await,
        Err(MailError::ValidationError(_))
    ));
    assert_eq!(outbox.lock().map(|sent| sent.len()).unwrap_or(0), 1);

    // 3. The live path against a loopback fixture: a receipt, a rate limit
    //    with its retry delay, then an outage that fails over.
    let endpoint = loopback_fixture(vec![
        "HTTP/1.1 202 Accepted\r\nContent-Type: application/json\r\nContent-Length: 16\r\nConnection: close\r\n\r\n{\"id\":\"msg_001\"}",
        "HTTP/1.1 429 Too Many Requests\r\nRetry-After: 7\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
    ])
    .await?;
    let live = AcmeMailDriver::try_new("live_fixture_key")?.try_with_endpoint(endpoint)?;
    assert_eq!(live.delivery_mode(), DeliveryMode::Real);
    live.send(&welcome("ada@example.com")).await?;
    let limited = live.send(&welcome("ada@example.com")).await;
    let Err(limited) = limited else {
        return Err("a 429 must fail".into());
    };
    assert_eq!(limited.failure_class(), MailFailureClass::RateLimited);
    assert_eq!(limited.retry_after(), Some(Duration::from_secs(7)));

    let (fallback, fallback_store) = MemoryDriver::isolated();
    let failover = FailoverDriver::new(live).with_fallback(fallback);
    failover.send(&welcome("ada@example.com")).await?;
    assert_eq!(fallback_store.lock().map(|sent| sent.len()).unwrap_or(0), 1);

    // ANCHOR: install
    // 4. Install the transport for the `Mail` facade (and its queue worker)...
    let transport = AcmeMailDriver::try_new("mock_acme")?;
    let facade_outbox = transport.offline_outbox();
    Mail::set_driver(Box::new(transport));
    Mail::send_now(welcome("grace@example.com")).await?;
    Mail::reset_driver();

    // ...or for one tenant, with the global default for everyone else.
    let (default_driver, default_store) = MemoryDriver::isolated();
    let resolver = TenantMailResolver::with_default(default_driver);
    resolver.register("tenant_acme", AcmeMailDriver::try_new("mock_tenant")?)?;
    resolver
        .send_for_tenant("tenant_acme", &welcome("linus@example.com"))
        .await?;
    resolver
        .send_for_tenant("tenant_other", &welcome("ken@example.com"))
        .await?;

    // Shared guards wrap a custom transport like a built-in one.
    let store = InMemorySuppressionStore::new(16, 16)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    store
        .record(SuppressionEvent::try_new(
            PROVIDER,
            "evt_bounce_1",
            "bounced@example.com",
            SuppressionReason::HardBounce,
            now,
        )?)
        .await?;
    let guarded = SuppressionGuard::new(AcmeMailDriver::try_new("mock_acme")?, store);
    assert!(matches!(
        guarded.send(&welcome("bounced@example.com")).await,
        Err(MailError::SuppressedRecipient { .. })
    ));
    // ANCHOR_END: install
    assert_eq!(facade_outbox.lock().map(|sent| sent.len()).unwrap_or(0), 1);
    assert_eq!(default_store.lock().map(|sent| sent.len()).unwrap_or(0), 1);

    println!(
        "AcmeMail transport: offline, live fixture, failover, facade, tenant and suppression checks passed"
    );
    Ok(())
}
