//! Alert delivery to a webhook the developer configures.
//!
//! The URL is a secret (it usually embeds the receiver's credential): it is
//! stored sealed like tokens and never shown. Only `https://` URLs are
//! accepted. Delivery posts a small JSON body, follows no redirect, ignores
//! proxy variables, is bounded to 5 seconds and is not retried; the outcome
//! is a fixed status recorded on the alert.

use super::projects::ProjectError;
use reqwest::Url;
use serde::Serialize;
use std::time::Duration;

const MAX_URL_CHARS: usize = 2_048;
const TIMEOUT: Duration = Duration::from_secs(5);

/// Validates a webhook URL: `https`, a host, no user information or fragment.
pub(crate) fn validate_webhook_url(value: &str) -> Result<Url, ProjectError> {
    let value = value.trim();
    if value.len() > MAX_URL_CHARS {
        return Err(ProjectError::InvalidWebhook);
    }
    let url = Url::parse(value).map_err(|_| ProjectError::InvalidWebhook)?;
    let valid = url.scheme() == "https"
        && url.host_str().is_some_and(|host| !host.is_empty())
        && url.username().is_empty()
        && url.password().is_none()
        && url.fragment().is_none();
    valid.then_some(url).ok_or(ProjectError::InvalidWebhook)
}

/// The body sent for one alert.
#[derive(Debug, Serialize)]
pub(crate) struct WebhookPayload<'a> {
    pub(crate) source: &'static str,
    pub(crate) project: &'a str,
    pub(crate) rule: &'a str,
    pub(crate) message: &'a str,
    pub(crate) at: u64,
}

/// Posts alerts to webhooks.
#[derive(Clone)]
pub(crate) struct WebhookClient {
    client: reqwest::Client,
}

impl WebhookClient {
    pub(crate) fn new() -> Option<Self> {
        Self::with_timeout(TIMEOUT)
    }

    pub(crate) fn with_timeout(timeout: Duration) -> Option<Self> {
        reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .timeout(timeout)
            .connect_timeout(Duration::from_secs(3))
            .user_agent(concat!("rullst-studio/", env!("CARGO_PKG_VERSION")))
            .build()
            .ok()
            .map(|client| Self { client })
    }

    /// Delivers `payload` once; the result is a fixed status.
    pub(crate) async fn deliver(&self, url: &str, payload: &WebhookPayload<'_>) -> String {
        let Ok(body) = serde_json::to_vec(payload) else {
            return "failed: could not encode the alert".to_string();
        };
        let outcome = self
            .client
            .post(url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body)
            .send()
            .await;
        match outcome {
            Ok(response) if response.status().is_success() => "delivered".to_string(),
            Ok(response) if response.status().is_redirection() => {
                "failed: redirect not followed".to_string()
            }
            Ok(response) => format!("failed: HTTP {}", response.status().as_u16()),
            Err(error) if error.is_timeout() => "failed: timeout".to_string(),
            Err(_) => "failed: unreachable".to_string(),
        }
    }
}
