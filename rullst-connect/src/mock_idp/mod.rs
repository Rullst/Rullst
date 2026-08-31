//! Bounded local OAuth 2.0 / OpenID Connect identity-provider fixture.
//!
//! The router is deliberately restricted to an HTTP loopback issuer and an
//! exact HTTP loopback redirect URI. Its Ed25519 signing key and credentials
//! are public, deterministic test fixtures. Never expose this router on a
//! production listener or treat it as an OIDC conformance implementation.

#[cfg(feature = "axum")]
mod handlers;
#[cfg(feature = "axum")]
mod signing;
#[cfg(all(test, feature = "axum"))]
mod tests;

#[cfg(feature = "axum")]
use secrecy::SecretString;
#[cfg(feature = "axum")]
use url::{Host, Url};

#[cfg(feature = "axum")]
pub const MOCK_IDP_CLIENT_ID: &str = "rullst-mock-client";
#[cfg(feature = "axum")]
pub const MOCK_IDP_CLIENT_SECRET: &str = "rullst-mock-secret";
#[cfg(feature = "axum")]
pub const MOCK_IDP_REDIRECT_URI: &str = "http://127.0.0.1:3000/callback";
#[cfg(feature = "axum")]
pub const MOCK_IDP_ISSUER: &str = "http://127.0.0.1:8080";

#[cfg(feature = "axum")]
const MAX_IDENTIFIER_BYTES: usize = 128;
#[cfg(feature = "axum")]
const MAX_PROFILE_FIELD_BYTES: usize = 320;

/// Static user returned by a [`MockIdpConfig`].
#[cfg(feature = "axum")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MockIdpUser {
    pub(crate) subject: String,
    pub(crate) name: String,
    pub(crate) email: String,
    pub(crate) picture: Option<String>,
}

#[cfg(feature = "axum")]
impl MockIdpUser {
    pub fn try_new(
        subject: impl Into<String>,
        name: impl Into<String>,
        email: impl Into<String>,
    ) -> Result<Self, crate::ConnectError> {
        let subject = bounded_field("mock_subject", subject.into(), MAX_IDENTIFIER_BYTES)?;
        let name = bounded_field("mock_name", name.into(), MAX_PROFILE_FIELD_BYTES)?;
        let email = bounded_field("mock_email", email.into(), MAX_PROFILE_FIELD_BYTES)?;
        if !email.contains('@') {
            return Err(crate::configuration::invalid(
                "mock_email",
                "must contain an @ separator",
            ));
        }
        Ok(Self {
            subject,
            name,
            email,
            picture: None,
        })
    }

    pub fn with_picture(mut self, picture: impl Into<String>) -> Result<Self, crate::ConnectError> {
        let picture = picture.into();
        let url = crate::configuration::validate_https_endpoint("mock_picture", &picture)?;
        self.picture = Some(url.to_string());
        Ok(self)
    }
}

#[cfg(feature = "axum")]
impl Default for MockIdpUser {
    fn default() -> Self {
        Self {
            subject: "rullst-mock-user".to_string(),
            name: "Rullst Mock User".to_string(),
            email: "mock@example.invalid".to_string(),
            picture: None,
        }
    }
}

/// Validated configuration for the explicitly mounted local IdP fixture.
#[cfg(feature = "axum")]
#[derive(Clone)]
pub struct MockIdpConfig {
    pub(crate) issuer: String,
    pub(crate) client_id: String,
    pub(crate) client_secret: SecretString,
    pub(crate) redirect_uri: String,
    pub(crate) user: MockIdpUser,
}

#[cfg(feature = "axum")]
impl MockIdpConfig {
    pub fn try_new(
        issuer: impl Into<String>,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        redirect_uri: impl Into<String>,
    ) -> Result<Self, crate::ConnectError> {
        let issuer = exact_loopback_url("mock_issuer", issuer.into(), true)?;
        if issuer.path() != "/" {
            return Err(crate::configuration::invalid(
                "mock_issuer",
                "must be an origin without a path",
            ));
        }
        let redirect_uri = exact_loopback_url("mock_redirect_uri", redirect_uri.into(), false)?;
        let client_id = bounded_field("mock_client_id", client_id.into(), MAX_IDENTIFIER_BYTES)?;
        let client_secret = bounded_field(
            "mock_client_secret",
            client_secret.into(),
            MAX_PROFILE_FIELD_BYTES,
        )?;

        Ok(Self {
            issuer: issuer.as_str().trim_end_matches('/').to_string(),
            client_id,
            client_secret: SecretString::from(client_secret),
            redirect_uri: redirect_uri.to_string(),
            user: MockIdpUser::default(),
        })
    }

    pub fn with_user(mut self, user: MockIdpUser) -> Self {
        self.user = user;
        self
    }

    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    pub fn redirect_uri(&self) -> &str {
        &self.redirect_uri
    }
}

#[cfg(feature = "axum")]
impl Default for MockIdpConfig {
    fn default() -> Self {
        Self {
            issuer: MOCK_IDP_ISSUER.to_string(),
            client_id: MOCK_IDP_CLIENT_ID.to_string(),
            client_secret: SecretString::from(MOCK_IDP_CLIENT_SECRET.to_string()),
            redirect_uri: MOCK_IDP_REDIRECT_URI.to_string(),
            user: MockIdpUser::default(),
        }
    }
}

/// Legacy authorization-query view retained for source compatibility.
#[cfg(feature = "axum")]
#[derive(serde::Deserialize)]
pub struct AuthQuery {
    pub client_id: String,
    pub redirect_uri: String,
    pub response_type: String,
    pub scope: Option<String>,
    pub state: Option<String>,
}

/// Legacy token-form view retained for source compatibility.
#[cfg(feature = "axum")]
#[derive(serde::Deserialize)]
pub struct TokenForm {
    pub client_id: String,
    pub client_secret: String,
    pub code: String,
    pub grant_type: String,
    pub redirect_uri: String,
}

/// Returns the deterministic default local IdP router.
#[cfg(feature = "axum")]
pub fn mock_router() -> axum::Router {
    mock_router_with_config(MockIdpConfig::default())
}

/// Returns a local IdP router for an already validated configuration.
#[cfg(feature = "axum")]
pub fn mock_router_with_config(config: MockIdpConfig) -> axum::Router {
    handlers::router(config)
}

#[cfg(feature = "axum")]
fn bounded_field(
    field: &'static str,
    value: String,
    maximum: usize,
) -> Result<String, crate::ConnectError> {
    if value.is_empty() || value.len() > maximum || value.chars().any(char::is_control) {
        return Err(crate::configuration::invalid(
            field,
            format!("must contain 1..={maximum} bytes without control characters"),
        ));
    }
    Ok(value)
}

#[cfg(feature = "axum")]
fn exact_loopback_url(
    field: &'static str,
    value: String,
    issuer: bool,
) -> Result<Url, crate::ConnectError> {
    let url = if issuer {
        crate::configuration::validate_issuer_url(&value)?
    } else {
        crate::configuration::validate_redirect_url(&value)?
    };
    let loopback = match url.host() {
        Some(Host::Domain(host)) => host.eq_ignore_ascii_case("localhost"),
        Some(Host::Ipv4(address)) => address.is_loopback(),
        Some(Host::Ipv6(address)) => address.is_loopback(),
        None => false,
    };
    if url.scheme() != "http" || !loopback {
        return Err(crate::configuration::invalid(
            field,
            "the mock IdP accepts only an HTTP loopback URL",
        ));
    }
    Ok(url)
}
