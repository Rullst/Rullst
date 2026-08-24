use std::sync::Arc;

use secrecy::SecretString;
use serde_json::{Value, json};

use crate::client::{HttpClient, HttpClientExt};
use crate::configuration::CredentialMode;
use crate::error::ConnectError;
use crate::provider::{JwksCache, JwksCachePolicy};

/// Provider created from a validated OpenID Connect discovery document.
pub struct OidcProvider {
    pub(crate) client_id: String,
    pub(crate) client_secret: SecretString,
    pub(crate) redirect_url: String,
    pub(crate) http_client: Arc<dyn HttpClient>,
    pub(crate) scopes: String,
    pub(crate) state: Option<String>,
    pub(crate) pkce_challenge: Option<String>,
    pub(crate) credential_mode: CredentialMode,
    pub(crate) jwks_cache: JwksCache,

    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub userinfo_endpoint: String,
    pub(crate) jwks_uri: String,
    pub issuer: String,
}

impl OidcProvider {
    /// Discovers and validates OIDC metadata. Empty or `mock_*` credentials
    /// construct deterministic local metadata and never perform network I/O.
    pub async fn discover(
        issuer_url: impl Into<String>,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        redirect_url: impl Into<String>,
    ) -> Result<Self, ConnectError> {
        let client: Arc<dyn HttpClient> = crate::client::DEFAULT_HTTP_CLIENT.clone();
        Self::discover_with_client(issuer_url, client_id, client_secret, redirect_url, client).await
    }

    /// Performs discovery with an injected client.
    pub(crate) async fn discover_with_client(
        issuer_url: impl Into<String>,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        redirect_url: impl Into<String>,
        client: Arc<dyn HttpClient>,
    ) -> Result<Self, ConnectError> {
        let issuer_url = issuer_url.into();
        let issuer_url = crate::configuration::validate_issuer_url(&issuer_url)?;
        let requested_issuer = crate::configuration::normalize_issuer(&issuer_url);

        let redirect_url = redirect_url.into();
        crate::configuration::validate_redirect_url(&redirect_url)?;

        let client_id = client_id.into();
        let client_secret = client_secret.into();
        let credential_mode =
            crate::configuration::credential_mode_for_values(&[&client_id, &client_secret]);

        let metadata = if credential_mode.is_mock() {
            mock_metadata(&requested_issuer)
        } else {
            let well_known_url = format!(
                "{}/.well-known/openid-configuration",
                requested_issuer.trim_end_matches('/')
            );
            client
                .get(well_known_url)
                .send()
                .await?
                .error_for_status()?
                .json::<Value>()
                .await?
        };

        let issuer = required_string(&metadata, "issuer")?;
        let discovered_issuer_url = crate::configuration::validate_issuer_url(&issuer)?;
        let discovered_issuer = crate::configuration::normalize_issuer(&discovered_issuer_url);
        if discovered_issuer != requested_issuer {
            return Err(ConnectError::InvalidConfiguration {
                field: "issuer",
                reason: format!(
                    "discovery issuer mismatch: requested '{requested_issuer}', received '{discovered_issuer}'"
                ),
            });
        }

        let authorization_endpoint =
            validated_endpoint(&metadata, "authorization_endpoint", &discovered_issuer_url)?;
        let token_endpoint =
            validated_endpoint(&metadata, "token_endpoint", &discovered_issuer_url)?;
        let userinfo_endpoint =
            validated_endpoint(&metadata, "userinfo_endpoint", &discovered_issuer_url)?;
        let jwks_uri = validated_endpoint(&metadata, "jwks_uri", &discovered_issuer_url)?;

        let http_client = if credential_mode.is_mock() {
            crate::configuration::provider_http_client(credential_mode, "oidc", None)
        } else {
            client
        };

        Ok(Self {
            client_id,
            client_secret: SecretString::from(client_secret),
            redirect_url,
            http_client,
            scopes: "openid profile email".to_string(),
            state: None,
            pkce_challenge: None,
            credential_mode,
            jwks_cache: JwksCache::default(),
            authorization_endpoint,
            token_endpoint,
            userinfo_endpoint,
            jwks_uri,
            issuer: discovered_issuer,
        })
    }

    /// Returns the selected credential mode.
    pub fn credential_mode(&self) -> CredentialMode {
        self.credential_mode
    }

    pub fn with_scopes(mut self, scopes: &[&str]) -> Self {
        self.scopes = scopes.join(" ");
        self
    }

    pub fn with_state(mut self, state: impl Into<String>) -> Self {
        self.state = Some(state.into());
        self
    }

    pub fn with_pkce(mut self, challenge: impl Into<String>) -> Self {
        self.pkce_challenge = Some(challenge.into());
        self
    }

    /// Configures a custom HTTP client for live credentials.
    /// Mock credentials retain their deterministic network-free transport.
    pub fn with_http_client(mut self, client: Arc<dyn HttpClient>) -> Self {
        if matches!(self.credential_mode, CredentialMode::Live) {
            self.http_client = client;
        }
        self
    }

    pub fn with_jwks_cache_policy(mut self, policy: JwksCachePolicy) -> Self {
        self.jwks_cache = JwksCache::new(policy);
        self
    }
}

fn required_string(metadata: &Value, field: &'static str) -> Result<String, ConnectError> {
    metadata
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| ConnectError::Provider(format!("Missing {field} in OIDC config")))
}

fn validated_endpoint(
    metadata: &Value,
    field: &'static str,
    issuer: &url::Url,
) -> Result<String, ConnectError> {
    let value = required_string(metadata, field)?;
    crate::configuration::validate_discovery_endpoint(field, &value, issuer)
        .map(|url| url.to_string())
}

fn mock_metadata(issuer: &str) -> Value {
    let issuer = issuer.trim_end_matches('/');
    json!({
        "issuer": issuer,
        "authorization_endpoint": format!("{issuer}/authorize"),
        "token_endpoint": format!("{issuer}/token"),
        "userinfo_endpoint": format!("{issuer}/userinfo"),
        "jwks_uri": format!("{issuer}/jwks")
    })
}
