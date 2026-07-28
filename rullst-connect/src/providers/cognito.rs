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
}

impl CognitoProvider {
    /// Note: domain should be the full base url, e.g., <https://my-domain.auth.us-east-1.amazoncognito.com>
    pub fn new(
        client_id: String,
        client_secret: secrecy::SecretString,
        redirect_url: String,
        domain: String,
    ) -> Self {
        use secrecy::ExposeSecret;
        assert!(
            !client_id.is_empty(),
            "Socialite Error: client_id cannot be empty"
        );
        assert!(
            !client_secret.expose_secret().is_empty(),
            "Socialite Error: client_secret cannot be empty"
        );
        assert!(
            redirect_url.starts_with("http"),
            "Socialite Error: redirect_url must be a valid HTTP/HTTPS URL"
        );
        let clean_domain = domain.trim_end_matches('/').to_string();
        Self {
            client_id,
            client_secret,
            redirect_url,
            domain: clean_domain,
            http_client: crate::client::DEFAULT_HTTP_CLIENT.clone(),
            scopes: "openid profile email".to_string(),
            state: None,
            pkce_challenge: None,
        }
    }

    /// Overrides the default scopes for this provider.
    pub fn with_scopes(mut self, scopes: &[&str]) -> Self {
        self.scopes = scopes.join(" ");
        self
    }

    /// Sets the state parameter for CSRF protection.
    pub fn with_state(mut self, state: &str) -> Self {
        self.state = Some(state.to_owned());
        self
    }

    pub fn with_pkce(mut self, challenge: &str) -> Self {
        self.pkce_challenge = Some(challenge.to_owned());
        self
    }

    pub fn with_http_client(
        mut self,
        client: ::std::sync::Arc<dyn crate::client::HttpClient>,
    ) -> Self {
        self.http_client = client;
        self
    }
}

#[async_trait]
impl Provider for CognitoProvider {
    fn redirect_url(&self) -> String {
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
