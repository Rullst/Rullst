//! Shared, fail-closed configuration validation for OAuth and OIDC providers.

use secrecy::{ExposeSecret, SecretString};
use url::{Host, Url};

use crate::client::{DisabledHttpClient, HttpClient, OfflineHttpClient};
use crate::error::ConnectError;

/// Describes how a provider obtains responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CredentialMode {
    /// Credentials are non-placeholder values and requests use the configured HTTP client.
    Live,
    /// Empty or `mock_*` credentials select the deterministic, network-free fallback.
    Mock,
    /// A deprecated infallible constructor received invalid configuration.
    Invalid,
}

impl CredentialMode {
    /// Returns `true` only for the deterministic network-free fallback.
    pub fn is_mock(self) -> bool {
        matches!(self, Self::Mock)
    }

    /// Returns `true` when a deprecated constructor failed closed.
    pub fn is_invalid(self) -> bool {
        matches!(self, Self::Invalid)
    }
}

pub(crate) fn credential_mode(client_id: &str, client_secret: &SecretString) -> CredentialMode {
    credential_mode_for_values(&[client_id, client_secret.expose_secret()])
}

pub(crate) fn credential_mode_for_values(values: &[&str]) -> CredentialMode {
    if values.iter().any(|value| is_mock_credential(value)) {
        CredentialMode::Mock
    } else {
        CredentialMode::Live
    }
}

fn is_mock_credential(value: &str) -> bool {
    let value = value.trim();
    value.is_empty() || value.to_ascii_lowercase().starts_with("mock_")
}

pub(crate) fn provider_http_client(
    mode: CredentialMode,
    provider: &'static str,
    invalid_reason: Option<String>,
) -> std::sync::Arc<dyn HttpClient> {
    match mode {
        CredentialMode::Live => crate::client::DEFAULT_HTTP_CLIENT.clone(),
        CredentialMode::Mock => std::sync::Arc::new(OfflineHttpClient::new(provider)),
        CredentialMode::Invalid => std::sync::Arc::new(DisabledHttpClient::new(
            invalid_reason.unwrap_or_else(|| "invalid provider configuration".to_string()),
        )),
    }
}

pub(crate) fn mock_redirect_url(
    provider: &str,
    state: Option<&str>,
    pkce_challenge: Option<&str>,
) -> String {
    let base = "https://example.invalid/rullst-connect/mock?".to_string();
    let start = base.len();
    let mut query = url::form_urlencoded::Serializer::for_suffix(base, start);
    query.append_pair("provider", provider);
    if let Some(state) = state {
        query.append_pair("state", state);
    }
    if let Some(challenge) = pkce_challenge {
        query.append_pair("code_challenge", challenge);
        query.append_pair("code_challenge_method", "S256");
    }
    query.finish()
}

pub(crate) fn validate_redirect_url(value: &str) -> Result<Url, ConnectError> {
    validate_https_or_loopback_url("redirect_url", value)
}

#[cfg(feature = "axum-session")]
pub(crate) fn validate_authorization_url(value: &str) -> Result<Url, ConnectError> {
    validate_https_or_loopback_url("authorization_url", value)
}

pub(crate) fn validate_jwks_url(value: &str) -> Result<Url, ConnectError> {
    validate_https_or_loopback_url("jwks_uri", value)
}

pub(crate) fn validate_issuer_url(value: &str) -> Result<Url, ConnectError> {
    let url = validate_https_or_loopback_url("issuer_url", value)?;
    if url.query().is_some() || url.fragment().is_some() {
        return Err(invalid(
            "issuer_url",
            "issuer URLs cannot contain a query string or fragment",
        ));
    }
    Ok(url)
}

pub(crate) fn validate_https_endpoint(
    field: &'static str,
    value: &str,
) -> Result<Url, ConnectError> {
    let url = parse_http_url(field, value)?;
    if url.scheme() != "https" {
        return Err(invalid(field, "endpoint must use HTTPS"));
    }
    reject_url_credentials_and_fragment(field, &url)?;
    Ok(url)
}

pub(crate) fn validate_discovery_endpoint(
    field: &'static str,
    value: &str,
    issuer: &Url,
) -> Result<Url, ConnectError> {
    let endpoint = parse_http_url(field, value)?;
    reject_url_credentials_and_fragment(field, &endpoint)?;

    if endpoint.scheme() == "https" {
        return Ok(endpoint);
    }

    if endpoint.scheme() == "http"
        && is_loopback(issuer)
        && is_loopback(&endpoint)
        && endpoint.host() == issuer.host()
        && endpoint.port_or_known_default() == issuer.port_or_known_default()
    {
        return Ok(endpoint);
    }

    Err(invalid(
        field,
        "discovered endpoints must use HTTPS; HTTP is limited to the exact issuer loopback origin",
    ))
}

pub(crate) fn validate_https_base_url(
    field: &'static str,
    value: &str,
) -> Result<String, ConnectError> {
    let mut url = validate_https_endpoint(field, value)?;
    if url.query().is_some() {
        return Err(invalid(field, "base URL cannot contain a query string"));
    }
    let trimmed = url.path().trim_end_matches('/').to_string();
    url.set_path(&trimmed);
    Ok(url.to_string().trim_end_matches('/').to_string())
}

pub(crate) fn validate_https_host(
    field: &'static str,
    value: &str,
) -> Result<String, ConnectError> {
    let candidate = format!("https://{}", value.trim().trim_end_matches('/'));
    let url = validate_https_endpoint(field, &candidate)?;
    if url.path() != "/" || url.query().is_some() {
        return Err(invalid(field, "must be a host name without path or query"));
    }
    let host = url
        .host_str()
        .ok_or_else(|| invalid(field, "must include a valid host"))?;
    Ok(match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_string(),
    })
}

pub(crate) fn normalize_issuer(url: &Url) -> String {
    url.as_str().trim_end_matches('/').to_string()
}

pub(crate) fn invalid(field: &'static str, reason: impl Into<String>) -> ConnectError {
    ConnectError::InvalidConfiguration {
        field,
        reason: reason.into(),
    }
}

fn validate_https_or_loopback_url(field: &'static str, value: &str) -> Result<Url, ConnectError> {
    let url = parse_http_url(field, value)?;
    reject_url_credentials_and_fragment(field, &url)?;
    if url.scheme() == "https" || (url.scheme() == "http" && is_loopback(&url)) {
        Ok(url)
    } else {
        Err(invalid(
            field,
            "must use HTTPS; HTTP is allowed only for the exact localhost, 127.0.0.1, or ::1 host",
        ))
    }
}

fn parse_http_url(field: &'static str, value: &str) -> Result<Url, ConnectError> {
    let url = Url::parse(value).map_err(|error| invalid(field, error.to_string()))?;
    if !matches!(url.scheme(), "http" | "https") || url.host().is_none() {
        return Err(invalid(field, "must be an absolute HTTP or HTTPS URL"));
    }
    Ok(url)
}

fn reject_url_credentials_and_fragment(field: &'static str, url: &Url) -> Result<(), ConnectError> {
    if !url.username().is_empty() || url.password().is_some() {
        return Err(invalid(field, "embedded URL credentials are not allowed"));
    }
    if url.fragment().is_some() {
        return Err(invalid(field, "URL fragments are not allowed"));
    }
    Ok(())
}

fn is_loopback(url: &Url) -> bool {
    match url.host() {
        Some(Host::Domain(host)) => host.eq_ignore_ascii_case("localhost"),
        Some(Host::Ipv4(address)) => address.is_loopback(),
        Some(Host::Ipv6(address)) => address.is_loopback(),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_lookalike_loopback_hosts() {
        for value in [
            "http://localhost.evil/callback",
            "http://127.0.0.1.evil/callback",
            "http://example.com/callback",
        ] {
            assert!(validate_redirect_url(value).is_err(), "accepted {value}");
        }
    }

    #[test]
    fn accepts_exact_loopback_and_https_urls() {
        for value in [
            "http://localhost:3000/callback",
            "http://127.0.0.1:3000/callback",
            "http://[::1]:3000/callback",
            "https://example.com/callback",
        ] {
            assert!(validate_redirect_url(value).is_ok(), "rejected {value}");
        }
    }

    #[test]
    fn credentials_select_an_explicit_mock_mode() {
        let secret = SecretString::from("real-secret".to_string());
        assert_eq!(credential_mode("", &secret), CredentialMode::Mock);
        assert_eq!(
            credential_mode("mock_client", &secret),
            CredentialMode::Mock
        );
        assert_eq!(credential_mode("client", &secret), CredentialMode::Live);
    }
}
