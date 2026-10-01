use crate::client::HttpClientExt;
use crate::error::ConnectError;
use crate::provider::Provider;
use crate::user::ConnectUser;
use async_trait::async_trait;
use serde_json::Value;

pub struct Auth0Provider {
    client_id: String,
    client_secret: secrecy::SecretString,
    redirect_url: String,
    domain: String,
    http_client: ::std::sync::Arc<dyn crate::client::HttpClient>,
    scopes: String,
    state: Option<String>,
    pkce_challenge: Option<String>,
    credential_mode: crate::configuration::CredentialMode,
}

impl Auth0Provider {
    /// Creates a provider for a tenant domain such as `dev-example.us.auth0.com`.
    pub fn try_new(
        client_id: impl Into<String>,
        client_secret: secrecy::SecretString,
        redirect_url: impl Into<String>,
        domain: impl Into<String>,
    ) -> Result<Self, ConnectError> {
        let client_id = client_id.into();
        let redirect_url = redirect_url.into();
        crate::configuration::validate_redirect_url(&redirect_url)?;
        let domain = crate::configuration::validate_https_host("domain", &domain.into())?;
        let credential_mode = crate::configuration::credential_mode(&client_id, &client_secret);
        let http_client =
            crate::configuration::provider_http_client(credential_mode, "auth0", None);

        Ok(Self {
            client_id,
            client_secret,
            redirect_url,
            domain,
            http_client,
            scopes: "openid profile email".to_string(),
            state: None,
            pkce_challenge: None,
            credential_mode,
        })
    }

    /// Deprecated infallible constructor. Invalid configuration is disabled.
    #[cfg_attr(
        not(test),
        deprecated(since = "12.0.0", note = "use try_new and handle ConnectError")
    )]
    pub fn new(
        client_id: impl Into<String>,
        client_secret: secrecy::SecretString,
        redirect_url: impl Into<String>,
        domain: impl Into<String>,
    ) -> Self {
        let client_id = client_id.into();
        let mut redirect_url = redirect_url.into();
        let raw_domain = domain.into();
        let redirect_result = crate::configuration::validate_redirect_url(&redirect_url);
        let domain_result = crate::configuration::validate_https_host("domain", &raw_domain);
        let (credential_mode, invalid_reason, domain) = match (redirect_result, domain_result) {
            (Ok(_), Ok(domain)) => (
                crate::configuration::credential_mode(&client_id, &client_secret),
                None,
                domain,
            ),
            (redirect, domain) => {
                redirect_url = "about:blank".to_string();
                let reason = match (redirect, domain) {
                    (Err(error), _) | (_, Err(error)) => error.to_string(),
                    (Ok(_), Ok(_)) => "invalid Auth0 configuration".to_string(),
                };
                (
                    crate::configuration::CredentialMode::Invalid,
                    Some(reason),
                    "invalid.invalid".to_string(),
                )
            }
        };
        let http_client =
            crate::configuration::provider_http_client(credential_mode, "auth0", invalid_reason);
        Self {
            client_id,
            client_secret,
            redirect_url,
            domain,
            http_client,
            scopes: "openid profile email".to_string(),
            state: None,
            pkce_challenge: None,
            credential_mode,
        }
    }

    pub fn credential_mode(&self) -> crate::configuration::CredentialMode {
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

    pub fn with_http_client(
        mut self,
        client: ::std::sync::Arc<dyn crate::client::HttpClient>,
    ) -> Self {
        if matches!(
            self.credential_mode,
            crate::configuration::CredentialMode::Live
        ) {
            self.http_client = client;
        }
        self
    }

    /// Replaces the provider's HTTP transport with a new direct `ReqwestClient` using
    /// the method-aware retry policy of `ReqwestClient::new_with_retry` (at most 10
    /// retries). Any client installed with `with_http_client`, including an explicit
    /// corporate proxy, is discarded. With the `retry` feature, the proxy constructors
    /// `ReqwestClient::try_with_proxy*` already apply that policy with three retries,
    /// so pass the proxy client last instead of calling this method.
    /// Only available with the `retry` feature.
    #[cfg(feature = "retry")]
    #[cfg_attr(mutants, mutants::skip)]
    pub fn with_retry(mut self, max_retries: u32) -> Self {
        if matches!(
            self.credential_mode,
            crate::configuration::CredentialMode::Live
        ) {
            self.http_client =
                ::std::sync::Arc::new(crate::client::ReqwestClient::new_with_retry(max_retries));
        }
        self
    }
}

#[async_trait]
impl Provider for Auth0Provider {
    fn redirect_url(&self) -> String {
        if self.credential_mode.is_invalid() {
            return "about:blank".to_string();
        }
        if self.credential_mode.is_mock() {
            return crate::configuration::mock_redirect_url(
                "auth0",
                self.state.as_deref(),
                self.pkce_challenge.as_deref(),
            );
        }
        let mut params = crate::provider::build_oauth_params(
            &format!("https://{}/authorize", self.domain),
            &self.client_id,
            &self.redirect_url,
            &self.scopes,
            self.state.as_deref(),
            self.pkce_challenge.as_deref(),
        );
        params.append_pair("response_type", "code");
        params.finish()
    }

    async fn get_user(
        &self,
        params: crate::provider::ExchangeParams<'_>,
    ) -> Result<crate::user::ConnectUser, crate::error::ConnectError> {
        let form_data = crate::provider::TokenExchangeForm {
            client_id: self.client_id.as_str(),
            client_secret: Some(secrecy::ExposeSecret::expose_secret(&self.client_secret)),
            code: params.auth_code,
            grant_type: Some("authorization_code"),
            redirect_uri: self.redirect_url.as_str(),
            code_verifier: params.code_verifier,
        };
        crate::provider::exchange_and_get_user(
            self,
            self.http_client.as_ref(),
            &self.token_url(),
            &form_data,
            params.expected_nonce,
        )
        .await
    }

    async fn get_user_from_token(&self, access_token: &str) -> Result<ConnectUser, ConnectError> {
        let user_res = self
            .http_client
            .get(format!("https://{}/userinfo", self.domain))
            .bearer_auth(access_token)
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;

        Ok(ConnectUser {
            id: user_res["sub"].as_str().map(String::from).ok_or_else(|| {
                crate::error::ConnectError::Provider("Missing user id".to_owned())
            })?,
            name: user_res["name"]
                .as_str()
                .map(String::from)
                .unwrap_or_default(),
            email: user_res["email"].as_str().map(String::from),
            avatar_url: user_res["picture"].as_str().map(String::from),
            email_verified: crate::user::email_verified_claim(&user_res["email_verified"]),
            raw_data: user_res,
            access_token: secrecy::SecretString::from(access_token.to_owned()),
            refresh_token: None,
            expires_in: None,
        })
    }

    fn token_url(&self) -> String {
        format!("https://{}/oauth/token", self.domain)
    }

    async fn refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<crate::user::ConnectUser, crate::error::ConnectError> {
        crate::provider::refresh_and_get_user(
            self,
            self.http_client.as_ref(),
            &self.token_url(),
            &self.client_id,
            &self.client_secret,
            refresh_token,
        )
        .await
    }

    async fn revoke_token_with_kind(
        &self,
        token: &str,
        kind: crate::provider::RevocationTokenKind,
    ) -> Result<(), ConnectError> {
        if kind != crate::provider::RevocationTokenKind::RefreshToken {
            return Err(crate::provider::unsupported_revocation_kind("Auth0", kind));
        }
        crate::provider::revoke_form_with_body_credentials(
            self.http_client.as_ref(),
            format!("https://{}/oauth/revoke", self.domain),
            &self.client_id,
            secrecy::ExposeSecret::expose_secret(&self.client_secret),
            token,
            None,
        )
        .await
    }
}

#[cfg(test)]
#[path = "auth0_tests.rs"]
mod tests;
