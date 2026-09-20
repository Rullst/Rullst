//! Native Container Apps/App Service managed identity token acquisition.

use crate::MailError;
use secrecy::{ExposeSecret, SecretString};

/// Short-lived token scoped to `https://communication.azure.com`.
pub struct AzureMailAccessToken {
    pub(super) value: SecretString,
    pub(super) expires_at: u64,
}

impl std::fmt::Debug for AzureMailAccessToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AzureMailAccessToken([REDACTED])")
    }
}

impl AzureMailAccessToken {
    pub fn new(token: impl Into<String>, expires_at: u64) -> Result<Self, MailError> {
        let token = token.into();
        if token.len() > 16384 || token.bytes().any(|b| !b.is_ascii_graphic()) {
            return Err(config());
        }
        Ok(Self {
            value: SecretString::from(token),
            expires_at,
        })
    }
}

/// Static-dispatch credential boundary; hosts can adapt an Azure SDK credential.
pub trait AzureMailCredential: Send + Sync {
    fn access_token(
        &self,
    ) -> impl std::future::Future<Output = Result<AzureMailAccessToken, MailError>> + Send;
}

/// Explicit supplied token, primarily for fixtures or host-owned token brokers.
/// Empty and `mock_*` values use deterministic offline delivery.
pub struct StaticAzureMailCredential {
    token: SecretString,
    expires_at: u64,
}

impl StaticAzureMailCredential {
    pub fn new(token: impl Into<String>, expires_at: u64) -> Result<Self, MailError> {
        let token = AzureMailAccessToken::new(token, expires_at)?;
        Ok(Self {
            token: token.value,
            expires_at,
        })
    }
}

impl AzureMailCredential for StaticAzureMailCredential {
    async fn access_token(&self) -> Result<AzureMailAccessToken, MailError> {
        AzureMailAccessToken::new(self.token.expose_secret(), self.expires_at)
    }
}

/// Container Apps/App Service local identity endpoint. No long-lived ACS key is
/// stored. Environment variables must be supplied by the trusted Azure host.
pub struct AzureManagedIdentity {
    endpoint: reqwest::Url,
    identity_header: SecretString,
    client_id: Option<String>,
}

impl AzureManagedIdentity {
    pub fn from_environment() -> Result<Self, MailError> {
        Self::new(
            std::env::var("IDENTITY_ENDPOINT").map_err(|_| config())?,
            std::env::var("IDENTITY_HEADER").map_err(|_| config())?,
            std::env::var("AZURE_CLIENT_ID").ok(),
        )
    }

    /// Restricts the secret identity header to a literal loopback address;
    /// redirects and proxies are disabled. Remote identity brokers need an
    /// explicitly implemented `AzureMailCredential` instead.
    pub fn new(
        endpoint: impl Into<String>,
        identity_header: impl Into<String>,
        client_id: Option<String>,
    ) -> Result<Self, MailError> {
        let endpoint = reqwest::Url::parse(&endpoint.into()).map_err(|_| config())?;
        let host = endpoint.host_str().ok_or_else(config)?;
        if endpoint.scheme() != "http"
            || !host
                .trim_matches(['[', ']'])
                .parse::<std::net::IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
        {
            return Err(config());
        }
        let identity_header = identity_header.into();
        if !(16..=4096).contains(&identity_header.len())
            || identity_header.bytes().any(|b| !b.is_ascii_graphic())
            || client_id.as_ref().is_some_and(|id| !super::valid_uuid(id))
        {
            return Err(config());
        }
        Ok(Self {
            endpoint,
            identity_header: SecretString::from(identity_header),
            client_id,
        })
    }
}

impl AzureMailCredential for AzureManagedIdentity {
    async fn access_token(&self) -> Result<AzureMailAccessToken, MailError> {
        let mut endpoint = self.endpoint.clone();
        endpoint
            .query_pairs_mut()
            .append_pair("resource", "https://communication.azure.com")
            .append_pair("api-version", "2019-08-01");
        if let Some(client_id) = &self.client_id {
            endpoint
                .query_pairs_mut()
                .append_pair("client_id", client_id);
        }
        let response = super::super::http::client()?
            .get(endpoint)
            .header("X-IDENTITY-HEADER", self.identity_header.expose_secret())
            .timeout(std::time::Duration::from_secs(3))
            .send()
            .await
            .map_err(|_| MailError::transport("azure-identity", "identity endpoint unavailable"))?;
        if !response.status().is_success() {
            return Err(MailError::ConfigError(
                "Azure managed identity rejected the token request".into(),
            ));
        }
        let response = super::read_json(response).await?;
        if response["token_type"]
            .as_str()
            .is_none_or(|kind| !kind.eq_ignore_ascii_case("bearer"))
        {
            return Err(config());
        }
        let expires = response["expires_on"]
            .as_u64()
            .or_else(|| response["expires_on"].as_str()?.parse().ok())
            .ok_or_else(config)?;
        let token = response["access_token"]
            .as_str()
            .filter(|token| !token.is_empty() && !token.starts_with("mock_"))
            .ok_or_else(config)?;
        AzureMailAccessToken::new(token, expires)
    }
}

fn config() -> MailError {
    MailError::ConfigError("invalid Azure mail credential configuration".into())
}
