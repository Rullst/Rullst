use crate::client::{HttpClient, HttpClientExt};
use crate::error::ConnectError;
use serde_json::Value;
use std::sync::Arc;

pub struct OidcProvider {
    pub(crate) client_id: String,
    pub(crate) client_secret: secrecy::SecretString,
    pub(crate) redirect_url: String,
    pub(crate) http_client: Arc<dyn HttpClient>,
    pub(crate) scopes: String,
    pub(crate) state: Option<String>,
    pub(crate) pkce_challenge: Option<String>,

    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub userinfo_endpoint: String,
    pub(crate) jwks_uri: String,
    pub issuer: String,
}

impl OidcProvider {
    /// Discovers the OIDC configuration from the issuer URL and creates a new provider.
    pub async fn discover(
        issuer_url: &str,
        client_id: String,
        client_secret: String,
        redirect_url: String,
    ) -> Result<Self, ConnectError> {
        let client: Arc<dyn HttpClient> = crate::client::DEFAULT_HTTP_CLIENT.clone();
        Self::discover_with_client(issuer_url, client_id, client_secret, redirect_url, client).await
    }

    /// Internal method that performs OIDC discovery using a provided HTTP client.
    /// This exists to enable injecting mock clients in tests.
    pub(crate) async fn discover_with_client(
        issuer_url: &str,
        client_id: String,
        client_secret: String,
        redirect_url: String,
        client: Arc<dyn HttpClient>,
    ) -> Result<Self, ConnectError> {
        if !issuer_url.starts_with("https://")
            && !issuer_url.starts_with("http://127.0.0.1")
            && !issuer_url.starts_with("http://localhost")
        {
            return Err(crate::error::ConnectError::Provider(
                "OIDC Error: issuer_url must be HTTPS (or localhost)".to_string(),
            ));
        }
        if !redirect_url.starts_with("https://")
            && !redirect_url.starts_with("http://127.0.0.1")
            && !redirect_url.starts_with("http://localhost")
        {
            return Err(crate::error::ConnectError::Provider(
                "OIDC Error: redirect_url must be HTTPS (or localhost)".to_string(),
            ));
        }
        if client_id.is_empty() {
            return Err(crate::error::ConnectError::Provider(
                "OIDC Error: client_id cannot be empty".to_string(),
            ));
        }
        if client_secret.is_empty() {
            return Err(crate::error::ConnectError::Provider(
                "OIDC Error: client_secret cannot be empty".to_string(),
            ));
        }

        let well_known_url = if issuer_url.ends_with('/') {
            format!("{}.well-known/openid-configuration", issuer_url)
        } else {
            format!("{}/.well-known/openid-configuration", issuer_url)
        };

        let res = client
            .get(&well_known_url)
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;

        let authorization_endpoint = res["authorization_endpoint"]
            .as_str()
            .ok_or_else(|| {
                crate::error::ConnectError::Provider(
                    "Missing authorization_endpoint in OIDC config".to_string(),
                )
            })?
            .to_string();

        let token_endpoint = res["token_endpoint"]
            .as_str()
            .ok_or_else(|| {
                crate::error::ConnectError::Provider(
                    "Missing token_endpoint in OIDC config".to_string(),
                )
            })?
            .to_string();

        let userinfo_endpoint = res["userinfo_endpoint"]
            .as_str()
            .ok_or_else(|| {
                crate::error::ConnectError::Provider(
                    "Missing userinfo_endpoint in OIDC config".to_string(),
                )
            })?
            .to_string();

        let jwks_uri = res["jwks_uri"]
            .as_str()
            .ok_or_else(|| {
                crate::error::ConnectError::Provider("Missing jwks_uri in OIDC config".to_string())
            })?
            .to_string();

        let issuer = res["issuer"]
            .as_str()
            .ok_or_else(|| {
                crate::error::ConnectError::Provider("Missing issuer in OIDC config".to_string())
            })?
            .to_string();

        Ok(Self {
            client_id,
            client_secret: client_secret.into(),
            redirect_url,
            http_client: client,
            scopes: "openid profile email".to_string(),
            state: None,
            pkce_challenge: None,
            authorization_endpoint,
            token_endpoint,
            userinfo_endpoint,
            jwks_uri,
            issuer,
        })
    }

    pub fn with_scopes(mut self, scopes: &[&str]) -> Self {
        self.scopes = scopes.join(" ");
        self
    }

    pub fn with_state(mut self, state: &str) -> Self {
        self.state = Some(state.to_owned());
        self
    }

    pub fn with_pkce(mut self, challenge: &str) -> Self {
        self.pkce_challenge = Some(challenge.to_owned());
        self
    }

    pub fn with_http_client(mut self, client: Arc<dyn HttpClient>) -> Self {
        self.http_client = client;
        self
    }
}
