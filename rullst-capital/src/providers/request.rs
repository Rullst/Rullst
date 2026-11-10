//! A request helper that gives application-owned payment adapters the HTTP
//! rules of Capital's built-in adapters without exposing an HTTP client type.

use crate::{CapitalError, ProviderFailure};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Largest request body a [`ProviderRequest`] sends.
const MAX_REQUEST_BYTES: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Method {
    Get,
    Post,
}

/// One HTTP request to a payment provider, sent with Capital's bounded client.
///
/// Application-owned [`BillingProvider`](super::BillingProvider) adapters use
/// it to get the rules of the built-in adapters without building their own
/// client: no redirects are followed, proxy variables are ignored, connecting
/// times out after 5 seconds and the whole request after 20, TLS uses rustls,
/// a response body is read up to 1 MiB, and failures are
/// [`ProviderFailure`] values that carry no URL, credential or body.
///
/// The URL must use HTTPS, or HTTP on a loopback host (`localhost`,
/// `127.0.0.1`, `[::1]`) for a local fixture, and must not contain
/// credentials. A non-2xx status is an error with the status and a bounded
/// `Retry-After`. The helper does not retry: a payment request is repeated only
/// with the provider's idempotency key and the application's reconciliation.
///
/// ```rust,no_run
/// use rullst_capital::providers::ProviderRequest;
///
/// # async fn check() -> Result<(), rullst_capital::CapitalError> {
/// let status: serde_json::Value = ProviderRequest::post_json(
///     "acmepay",
///     "payment check",
///     "https://api.acmepay.example/v1/payments/check",
///     &serde_json::json!({ "order": "order_42" }),
/// )?
/// .bearer_auth("server-side key")
/// .send_json()
/// .await?;
/// # let _ = status;
/// # Ok(())
/// # }
/// ```
#[non_exhaustive]
pub struct ProviderRequest {
    provider: &'static str,
    operation: &'static str,
    method: Method,
    url: String,
    headers: Vec<(String, String)>,
    body: Option<(Vec<u8>, &'static str)>,
}

impl ProviderRequest {
    /// A `GET` request. `provider` and `operation` are static labels that
    /// name the failure in errors and telemetry.
    pub fn get(
        provider: &'static str,
        operation: &'static str,
        url: impl Into<String>,
    ) -> Result<Self, CapitalError> {
        Self::new(provider, operation, Method::Get, url.into())
    }

    /// A `POST` request with a JSON body.
    pub fn post_json<T: Serialize + ?Sized>(
        provider: &'static str,
        operation: &'static str,
        url: impl Into<String>,
        body: &T,
    ) -> Result<Self, CapitalError> {
        let body = serde_json::to_vec(body)
            .map_err(|_| ProviderFailure::request_build(provider, operation))?;
        Self::new(provider, operation, Method::Post, url.into())?
            .with_body(body, "application/json")
    }

    /// A `POST` request with an `application/x-www-form-urlencoded` body the
    /// caller already encoded (see [`url_encode`](super::url_encode)).
    pub fn post_form(
        provider: &'static str,
        operation: &'static str,
        url: impl Into<String>,
        body: impl Into<String>,
    ) -> Result<Self, CapitalError> {
        Self::new(provider, operation, Method::Post, url.into())?.with_body(
            body.into().into_bytes(),
            "application/x-www-form-urlencoded",
        )
    }

    /// Adds a header. An invalid name or value fails when the request is sent.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Adds `Authorization: Bearer <token>`.
    pub fn bearer_auth(self, token: impl Into<String>) -> Self {
        let token = token.into();
        self.header("Authorization", format!("Bearer {token}"))
    }

    /// Sends the request and decodes a successful JSON response.
    pub async fn send_json<T: DeserializeOwned>(self) -> Result<T, CapitalError> {
        let (provider, operation) = (self.provider, self.operation);
        let response = self.execute().await?;
        super::read_http_json(response, provider, operation).await
    }

    /// Sends the request and returns a successful response body (at most 1 MiB).
    pub async fn send(self) -> Result<Vec<u8>, CapitalError> {
        let (provider, operation) = (self.provider, self.operation);
        let response = self.execute().await?;
        super::http::read_bytes(response, provider, operation).await
    }

    fn new(
        provider: &'static str,
        operation: &'static str,
        method: Method,
        url: String,
    ) -> Result<Self, CapitalError> {
        let valid = reqwest::Url::parse(&url).ok().is_some_and(|parsed| {
            url.len() <= 2048
                && parsed.host_str().is_some()
                && parsed.username().is_empty()
                && parsed.password().is_none()
                && (parsed.scheme() == "https" || super::http::is_loopback_http(&parsed))
        });
        if !valid {
            return Err(ProviderFailure::request_build(provider, operation).into());
        }
        Ok(Self {
            provider,
            operation,
            method,
            url,
            headers: Vec::new(),
            body: None,
        })
    }

    fn with_body(
        mut self,
        body: Vec<u8>,
        content_type: &'static str,
    ) -> Result<Self, CapitalError> {
        if body.len() > MAX_REQUEST_BYTES {
            return Err(ProviderFailure::request_build(self.provider, self.operation).into());
        }
        self.body = Some((body, content_type));
        Ok(self)
    }

    async fn execute(self) -> Result<reqwest::Response, CapitalError> {
        let client = super::http_client()?;
        let mut builder = match self.method {
            Method::Get => client.get(&self.url),
            Method::Post => client.post(&self.url),
        };
        if let Some((body, content_type)) = self.body {
            builder = builder.header("Content-Type", content_type).body(body);
        }
        for (name, value) in &self.headers {
            builder = builder.header(name.as_str(), value.as_str());
        }
        super::send_http(builder, self.provider, self.operation).await
    }
}

impl std::fmt::Debug for ProviderRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProviderRequest")
            .field("provider", &self.provider)
            .field("operation", &self.operation)
            .field("method", &self.method)
            .field("url", &"[REDACTED]")
            .field("headers", &self.headers.len())
            .field(
                "body_bytes",
                &self.body.as_ref().map(|(body, _)| body.len()),
            )
            .finish()
    }
}

#[cfg(test)]
#[path = "request_tests.rs"]
mod tests;
