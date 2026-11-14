use crate::{CapitalError, ProviderFailure};
use reqwest::{RequestBuilder, Response, redirect::Policy};
use serde::de::DeserializeOwned;
use std::{sync::OnceLock, time::Duration};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
const POOL_IDLE_TIMEOUT: Duration = Duration::from_secs(90);
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_RETRY_AFTER: Duration = Duration::from_secs(24 * 60 * 60);
const MAX_PROVIDER_URL_BYTES: usize = 2048;

static HTTP_CLIENT: OnceLock<Result<reqwest::Client, CapitalError>> = OnceLock::new();

pub(crate) fn client() -> Result<&'static reqwest::Client, CapitalError> {
    match HTTP_CLIENT.get_or_init(build_client) {
        Ok(client) => Ok(client),
        Err(error) => Err(error.clone()),
    }
}

fn build_client() -> Result<reqwest::Client, CapitalError> {
    reqwest::Client::builder()
        .redirect(Policy::none())
        .no_proxy()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .pool_idle_timeout(POOL_IDLE_TIMEOUT)
        .build()
        .map_err(|_| {
            CapitalError::ConfigurationError(
                "failed to construct the bounded payment-provider HTTP client".to_string(),
            )
        })
}

pub(crate) async fn send(
    request: RequestBuilder,
    provider: &'static str,
    operation: &'static str,
) -> Result<Response, CapitalError> {
    let request = request
        .build()
        .map_err(|_| ProviderFailure::request_build(provider, operation))?;
    execute(request, provider, operation).await
}

pub(crate) async fn execute(
    request: reqwest::Request,
    provider: &'static str,
    operation: &'static str,
) -> Result<Response, CapitalError> {
    let response = client()?
        .execute(request)
        .await
        .map_err(|_| ProviderFailure::transport(provider, operation))?;
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    let retry_after = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(parse_retry_after);
    let reason = error_body(response).await;
    Err(
        ProviderFailure::http_response(provider, operation, status.as_u16(), retry_after)
            .with_reason(reason.as_deref().and_then(error_code).as_deref())
            .into(),
    )
}

/// At most this many bytes of an unsuccessful response are read, only to find
/// its error code.
const MAX_ERROR_BODY_BYTES: usize = 4096;

/// The body of an unsuccessful response when it is small; `None` otherwise.
async fn error_body(mut response: Response) -> Option<Vec<u8>> {
    if response
        .content_length()
        .is_some_and(|length| length > MAX_ERROR_BODY_BYTES as u64)
    {
        return None;
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.ok()? {
        if body.len().saturating_add(chunk.len()) > MAX_ERROR_BODY_BYTES {
            return None;
        }
        body.extend_from_slice(&chunk);
    }
    Some(body)
}

/// The provider's error code in a JSON error body: a string `error` (as
/// InfinitePay sends) or `error.code`, else `error.type` (as Stripe sends).
/// [`ProviderFailure`] keeps it only when it is a short token.
pub(crate) fn error_code(body: &[u8]) -> Option<String> {
    let value: serde_json::Value = serde_json::from_slice(body).ok()?;
    let error = value.get("error")?;
    error
        .as_str()
        .or_else(|| {
            ["code", "type"]
                .iter()
                .find_map(|field| error.get(*field).and_then(serde_json::Value::as_str))
        })
        .map(str::to_owned)
}

pub(crate) async fn send_json<T: DeserializeOwned>(
    request: RequestBuilder,
    provider: &'static str,
    operation: &'static str,
) -> Result<T, CapitalError> {
    let response = send(request, provider, operation).await?;
    read_json(response, provider, operation).await
}

pub(crate) async fn read_json<T: DeserializeOwned>(
    response: Response,
    provider: &'static str,
    operation: &'static str,
) -> Result<T, CapitalError> {
    let body = read_bytes(response, provider, operation).await?;
    serde_json::from_slice(&body)
        .map_err(|_| ProviderFailure::invalid_response(provider, operation).into())
}

/// Reads a successful response body of at most 1 MiB.
pub(crate) async fn read_bytes(
    mut response: Response,
    provider: &'static str,
    operation: &'static str,
) -> Result<Vec<u8>, CapitalError> {
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        return Err(ProviderFailure::response_too_large(provider, operation).into());
    }

    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| ProviderFailure::transport(provider, operation))?
    {
        if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            return Err(ProviderFailure::response_too_large(provider, operation).into());
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

pub(crate) fn validate_checkout_url(
    provider: &'static str,
    value: &str,
) -> Result<String, CapitalError> {
    let parsed = parse_bounded_url(value);
    let valid = parsed.as_ref().is_some_and(|url| {
        url.scheme() == "https"
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            // Stripe's hosted checkout URL contains an opaque client fragment.
            // Preserve it instead of rejecting or stripping provider state.
            && (provider == "stripe" || url.fragment().is_none())
    });
    if !valid {
        return Err(ProviderFailure::contract_mismatch(provider, "create checkout").into());
    }
    Ok(value.to_string())
}

/// Validates a success or cancel URL when a checkout request is built.
///
/// It accepts a bounded, credential-free URL without a fragment that uses
/// HTTPS, or HTTP on a loopback host (`localhost`, `127.0.0.1`, `[::1]`) so a
/// `mock_*` key works with a local development server. A live key requires
/// HTTPS when the request is dispatched ([`require_https_redirects`]).
pub(crate) fn validate_redirect_url(value: &str) -> Result<(), CapitalError> {
    let valid = parse_bounded_url(value).is_some_and(|url| {
        url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.fragment().is_none()
            && (url.scheme() == "https" || is_loopback_http(&url))
    });
    if valid {
        Ok(())
    } else {
        Err(CapitalError::ConfigurationError(
            "checkout redirects require bounded credential-free HTTPS URLs without fragments (HTTP only on localhost with a mock key)".into(),
        ))
    }
}

/// Rejects loopback `http://` redirects before a live provider request.
pub(crate) fn require_https_redirects(urls: &[&str]) -> Result<(), CapitalError> {
    if urls
        .iter()
        .all(|url| parse_bounded_url(url).is_some_and(|url| url.scheme() == "https"))
    {
        Ok(())
    } else {
        Err(CapitalError::ConfigurationError(
            "a live provider key requires HTTPS success and cancel URLs; http://localhost works only with a mock_* key".into(),
        ))
    }
}

pub(crate) fn is_loopback_http(url: &reqwest::Url) -> bool {
    url.scheme() == "http"
        && url.host_str().is_some_and(|host| {
            host.eq_ignore_ascii_case("localhost")
                || host
                    .trim_start_matches('[')
                    .trim_end_matches(']')
                    .parse::<std::net::IpAddr>()
                    .is_ok_and(|address| address.is_loopback())
        })
}

fn parse_bounded_url(value: &str) -> Option<reqwest::Url> {
    (value.len() <= MAX_PROVIDER_URL_BYTES
        && !value
            .chars()
            .any(|character| character.is_whitespace() || character.is_control()))
    .then(|| reqwest::Url::parse(value).ok())
    .flatten()
}

fn parse_retry_after(value: &str) -> Option<Duration> {
    value
        .trim()
        .parse::<u64>()
        .ok()
        .map(Duration::from_secs)
        .map(|duration| duration.min(MAX_RETRY_AFTER))
}

#[cfg(test)]
#[path = "http_tests.rs"]
mod tests;
