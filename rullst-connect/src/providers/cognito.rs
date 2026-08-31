use crate::client::HttpClientExt;
use crate::error::ConnectError;
use crate::provider::Provider;
use crate::user::ConnectUser;
use async_trait::async_trait;
use serde_json::Value;

pub struct CognitoProvider {
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

impl CognitoProvider {
    /// Note: domain should be the full base url, e.g., <https://my-domain.auth.us-east-1.amazoncognito.com>
    pub fn try_new(
        client_id: impl Into<String>,
        client_secret: secrecy::SecretString,
        redirect_url: impl Into<String>,
        domain: impl Into<String>,
    ) -> Result<Self, ConnectError> {
        let client_id = client_id.into();
        let redirect_url = redirect_url.into();
        crate::configuration::validate_redirect_url(&redirect_url)?;
        let domain = crate::configuration::validate_https_base_url("domain", &domain.into())?;
        let credential_mode = crate::configuration::credential_mode(&client_id, &client_secret);
        let http_client =
            crate::configuration::provider_http_client(credential_mode, "cognito", None);

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
        let domain_result = crate::configuration::validate_https_base_url("domain", &raw_domain);
        let (credential_mode, invalid_reason, clean_domain) = match (redirect_result, domain_result)
        {
            (Ok(_), Ok(domain)) => (
                crate::configuration::credential_mode(&client_id, &client_secret),
                None,
                domain,
            ),
            (redirect, domain) => {
                redirect_url = "about:blank".to_string();
                let reason = match (redirect, domain) {
                    (Err(error), _) | (_, Err(error)) => error.to_string(),
                    (Ok(_), Ok(_)) => "invalid Cognito configuration".to_string(),
                };
                (
                    crate::configuration::CredentialMode::Invalid,
                    Some(reason),
                    "https://invalid.invalid".to_string(),
                )
            }
        };
        let http_client =
            crate::configuration::provider_http_client(credential_mode, "cognito", invalid_reason);
        Self {
            client_id,
            client_secret,
            redirect_url,
            domain: clean_domain,
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

    /// Overrides the default scopes for this provider.
    pub fn with_scopes(mut self, scopes: &[&str]) -> Self {
        self.scopes = scopes.join(" ");
        self
    }

    /// Sets the state parameter for CSRF protection.
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
}

#[async_trait]
impl Provider for CognitoProvider {
    fn redirect_url(&self) -> String {
        if self.credential_mode.is_invalid() {
            return "about:blank".to_string();
        }
        if self.credential_mode.is_mock() {
            return crate::configuration::mock_redirect_url(
                "cognito",
                self.state.as_deref(),
                self.pkce_challenge.as_deref(),
            );
        }
        let mut params = crate::provider::build_oauth_params(
            &format!("{}/oauth2/authorize", self.domain),
            &self.client_id,
            &self.redirect_url,
            &self.scopes,
            self.state.as_deref(),
            self.pkce_challenge.as_deref(),
        );
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
            .get(format!("{}/oauth2/userInfo", self.domain))
            .bearer_auth(access_token)
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;

        Ok(ConnectUser {
            id: user_res["sub"].as_str().map(String::from).ok_or_else(|| {
                crate::error::ConnectError::Provider("Missing user id".to_string())
            })?,
            name: user_res["name"]
                .as_str()
                .or_else(|| user_res["username"].as_str())
                .map(String::from)
                .unwrap_or_default(),
            email: user_res["email"].as_str().map(String::from),
            avatar_url: user_res["picture"].as_str().map(String::from),
            email_verified: None,
            raw_data: user_res,
            access_token: secrecy::SecretString::from(access_token.to_string()),
            refresh_token: None,
            expires_in: None,
        })
    }

    fn token_url(&self) -> String {
        format!("{}/oauth2/token", self.domain)
    }

    async fn refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<crate::user::ConnectUser, crate::error::ConnectError> {
        let token = crate::provider::fetch_refresh_token(
            self.http_client.as_ref(),
            &self.token_url(),
            &self.client_id,
            secrecy::ExposeSecret::expose_secret(&self.client_secret),
            refresh_token,
        )
        .await?;

        let mut user = self.get_user_from_token(&token.access_token).await?;
        user.refresh_token = token.refresh_token.map(secrecy::SecretString::from);
        user.expires_in = token.expires_in;
        Ok(user)
    }

    async fn revoke_token_with_kind(
        &self,
        token: &str,
        kind: crate::provider::RevocationTokenKind,
    ) -> Result<(), ConnectError> {
        if kind != crate::provider::RevocationTokenKind::RefreshToken {
            return Err(crate::provider::unsupported_revocation_kind(
                "Amazon Cognito",
                kind,
            ));
        }
        crate::provider::revoke_form_with_basic_credentials(
            self.http_client.as_ref(),
            format!("{}/oauth2/revoke", self.domain),
            &self.client_id,
            secrecy::ExposeSecret::expose_secret(&self.client_secret),
            token,
            None,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cognito_redirect_url() {
        let provider = CognitoProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect.url".to_string(),
            "https://my-domain.auth.us-east-1.amazoncognito.com".to_string(),
        );

        let url = provider.redirect_url();
        assert!(
            url.starts_with("https://my-domain.auth.us-east-1.amazoncognito.com/oauth2/authorize?")
        );
        assert!(url.contains("client_id=client_id"));
        assert!(url.contains("redirect_uri=https%3A%2F%2Fredirect.url"));
    }

    #[tokio::test]
    async fn test_cognito_mock_credentials_are_network_free() {
        let provider = CognitoProvider::try_new(
            "mock_client",
            secrecy::SecretString::from("mock_secret".to_string()),
            "https://app.example/callback",
            "https://tenant.auth.us-east-1.amazoncognito.com",
        )
        .expect("mock provider");
        let user = provider
            .get_user(crate::provider::ExchangeParams::default())
            .await
            .expect("offline user");
        assert_eq!(user.id, "mock-user");
        assert_eq!(
            provider.credential_mode(),
            crate::configuration::CredentialMode::Mock
        );
    }

    use crate::client::{HttpClient, HttpRequest, HttpResponse};
    use serde_json::json;
    use std::sync::Arc;

    struct MockCognitoClient {
        token_status: u16,
        token_body: serde_json::Value,
        user_status: u16,
        user_body: serde_json::Value,
    }

    #[async_trait]
    impl HttpClient for MockCognitoClient {
        async fn execute(
            &self,
            req: HttpRequest,
        ) -> Result<HttpResponse, crate::error::ConnectError> {
            if req.url.contains("token") {
                Ok(HttpResponse {
                    status: self.token_status,
                    body: self.token_body.clone(),
                })
            } else if req.url.contains("userInfo") {
                Ok(HttpResponse {
                    status: self.user_status,
                    body: self.user_body.clone(),
                })
            } else {
                Err(crate::error::ConnectError::Provider(
                    "Unexpected URL".to_string(),
                ))
            }
        }
    }

    #[tokio::test]
    async fn test_cognito_get_user_success() {
        let provider = CognitoProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect.url".to_string(),
            "https://my-domain.auth.us-east-1.amazoncognito.com".to_string(),
        )
        .with_http_client(Arc::new(MockCognitoClient {
            token_status: 200,
            token_body: json!({
                "access_token": "mock_access_token",
                "expires_in": 3600
            }),
            user_status: 200,
            user_body: json!({
                "sub": "user_123",
                "name": "Test User",
                "email": "test@example.com",
                "picture": "https://avatar.url"
            }),
        }));

        let user = provider
            .get_user(crate::provider::ExchangeParams {
                auth_code: "code",
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(user.id, "user_123");
        assert_eq!(user.name, "Test User");
        assert_eq!(user.email.as_deref(), Some("test@example.com"));
    }

    #[tokio::test]
    async fn test_cognito_token_error() {
        let provider = CognitoProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect.url".to_string(),
            "https://my-domain.auth.us-east-1.amazoncognito.com".to_string(),
        )
        .with_http_client(Arc::new(MockCognitoClient {
            token_status: 400,
            token_body: json!({"error": "invalid_grant"}),
            user_status: 200,
            user_body: json!({}),
        }));

        let err = provider
            .get_user(crate::provider::ExchangeParams {
                auth_code: "code",
                ..Default::default()
            })
            .await
            .unwrap_err();

        assert!(matches!(
            err,
            crate::error::ConnectError::ProviderApiError { .. }
        ));
    }

    #[tokio::test]
    async fn test_cognito_missing_id() {
        let provider = CognitoProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect.url".to_string(),
            "https://my-domain.auth.us-east-1.amazoncognito.com".to_string(),
        )
        .with_http_client(Arc::new(MockCognitoClient {
            token_status: 200,
            token_body: json!({"access_token": "mock_access_token"}),
            user_status: 200,
            user_body: json!({"name": "No ID User"}),
        }));

        let err = provider
            .get_user(crate::provider::ExchangeParams {
                auth_code: "code",
                ..Default::default()
            })
            .await
            .unwrap_err();

        assert!(matches!(err, crate::error::ConnectError::Provider(_)));
    }

    #[tokio::test]
    async fn test_cognito_refresh_token_success() {
        let provider = CognitoProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect.url".to_string(),
            "https://my-domain.auth.us-east-1.amazoncognito.com".to_string(),
        )
        .with_http_client(Arc::new(MockCognitoClient {
            token_status: 200,
            token_body: json!({
                "access_token": "new_access_token",
                "refresh_token": "new_refresh_token",
                "expires_in": 3600
            }),
            user_status: 200,
            user_body: json!({
                "sub": "user_123",
                "username": "Test User Refreshed",
                "email": "test@example.com",
                "picture": "https://avatar.url"
            }),
        }));

        let user = provider.refresh_token("old_refresh").await.unwrap();
        assert_eq!(user.id, "user_123");
        assert_eq!(user.name, "Test User Refreshed");
        use secrecy::ExposeSecret;
        assert_eq!(
            user.refresh_token.unwrap().expose_secret(),
            "new_refresh_token"
        );
    }
}
