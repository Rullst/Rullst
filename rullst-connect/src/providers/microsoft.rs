use crate::client::HttpClientExt;
use crate::provider::Provider;
use crate::user::ConnectUser;
use async_trait::async_trait;
use serde_json::Value;

crate::define_provider!(MicrosoftProvider, "User.Read");

#[async_trait]
impl Provider for MicrosoftProvider {
    crate::impl_standard_redirect_url!(
        "https://login.microsoftonline.com/common/oauth2/v2.0/authorize"
    );

    async fn get_user(
        &self,
        params: crate::provider::ExchangeParams<'_>,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
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

    async fn get_user_from_token(
        &self,
        access_token: &str,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
        let user_res = self
            .http_client
            .get("https://graph.microsoft.com/v1.0/me")
            .bearer_auth(access_token)
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;

        Ok(ConnectUser {
            id: user_res["id"].as_str().map(String::from).ok_or_else(|| {
                crate::error::ConnectError::Provider("Missing user id".to_string())
            })?,
            name: user_res["displayName"]
                .as_str()
                .map(String::from)
                .ok_or_else(|| {
                    crate::error::ConnectError::Provider("Missing user displayName".to_string())
                })?,
            email: user_res["mail"]
                .as_str()
                .or_else(|| user_res["userPrincipalName"].as_str())
                .map(String::from),
            avatar_url: None, // Requires a separate request to /me/photo/$value
            email_verified: None,
            raw_data: user_res,
            access_token: secrecy::SecretString::from(access_token.to_string()),
            refresh_token: None,
            expires_in: None,
        })
    }

    fn token_url(&self) -> String {
        "https://login.microsoftonline.com/common/oauth2/v2.0/token".to_string()
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{HttpClient, HttpRequest, HttpResponse};
    use serde_json::json;
    use std::sync::Arc;

    struct MockMicrosoftClient {
        token_status: u16,
        token_body: serde_json::Value,
        user_status: u16,
        user_body: serde_json::Value,
    }

    #[async_trait]
    impl HttpClient for MockMicrosoftClient {
        async fn execute(
            &self,
            req: HttpRequest,
        ) -> Result<HttpResponse, crate::error::ConnectError> {
            if req.url.contains("token") {
                Ok(HttpResponse {
                    status: self.token_status,
                    body: self.token_body.clone(),
                })
            } else if req.url.contains("me") {
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
    async fn test_microsoft_get_user_success() {
        let provider = MicrosoftProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect.url".to_string(),
        )
        .with_http_client(Arc::new(MockMicrosoftClient {
            token_status: 200,
            token_body: json!({
                "access_token": "mock_access_token",
                "expires_in": 3600
            }),
            user_status: 200,
            user_body: json!({
                "id": "12345",
                "displayName": "Test User",
                "mail": "test@example.com"
            }),
        }));

        let user = provider
            .get_user(crate::provider::ExchangeParams {
                auth_code: "code",
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(user.id, "12345");
        assert_eq!(user.name, "Test User");
        assert_eq!(user.email.as_deref(), Some("test@example.com"));
    }

    #[tokio::test]
    async fn test_microsoft_token_error() {
        let provider = MicrosoftProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect.url".to_string(),
        )
        .with_http_client(Arc::new(MockMicrosoftClient {
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
    async fn test_microsoft_missing_id() {
        let provider = MicrosoftProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect.url".to_string(),
        )
        .with_http_client(Arc::new(MockMicrosoftClient {
            token_status: 200,
            token_body: json!({"access_token": "mock_access_token"}),
            user_status: 200,
            user_body: json!({"displayName": "No ID User"}),
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
    async fn test_microsoft_missing_display_name() {
        let provider = MicrosoftProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect.url".to_string(),
        )
        .with_http_client(Arc::new(MockMicrosoftClient {
            token_status: 200,
            token_body: json!({"access_token": "mock_access_token"}),
            user_status: 200,
            user_body: json!({"id": "12345"}),
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
    async fn test_microsoft_refresh_token_success() {
        let provider = MicrosoftProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect.url".to_string(),
        )
        .with_http_client(Arc::new(MockMicrosoftClient {
            token_status: 200,
            token_body: json!({
                "access_token": "new_access_token",
                "refresh_token": "new_refresh_token",
                "expires_in": 3600
            }),
            user_status: 200,
            user_body: json!({
                "id": "12345",
                "displayName": "Test User Refreshed",
                "mail": "test@example.com"
            }),
        }));

        let user = provider.refresh_token("old_refresh").await.unwrap();
        assert_eq!(user.id, "12345");
        assert_eq!(user.name, "Test User Refreshed");
        use secrecy::ExposeSecret;
        assert_eq!(
            user.refresh_token.unwrap().expose_secret(),
            "new_refresh_token"
        );
    }
}
