use crate::client::HttpClientExt;
use crate::provider::Provider;
use crate::user::ConnectUser;
use async_trait::async_trait;
use serde_json::Value;

crate::define_provider!(FacebookProvider, "email", "public_profile");

impl FacebookProvider {
    async fn get_user_from_form(
        &self,
        form_data: Vec<(&str, &str)>,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
        let token_res = self
            .http_client
            .post(self.token_url())
            .form(&form_data)
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;

        let access_token = token_res["access_token"].as_str().ok_or_else(|| {
            crate::error::ConnectError::Token("Failed to get access_token".to_string())
        })?;

        let mut user = self.get_user_from_token(access_token).await?;
        user.refresh_token = token_res["refresh_token"]
            .as_str()
            .map(|s| secrecy::SecretString::from(s.to_string()));
        user.expires_in = crate::provider::token_lifetime(&token_res)?;
        Ok(user)
    }
}

#[async_trait]
impl Provider for FacebookProvider {
    crate::impl_standard_redirect_url!("https://www.facebook.com/v19.0/dialog/oauth");

    async fn get_user(
        &self,
        params: crate::provider::ExchangeParams<'_>,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
        crate::provider::require_oauth_only(params.expected_nonce)?;
        let mut form_data = vec![
            ("client_id", self.client_id.as_str()),
            (
                "client_secret",
                secrecy::ExposeSecret::expose_secret(&self.client_secret),
            ),
            ("code", params.auth_code),
            ("redirect_uri", self.redirect_url.as_str()),
        ];
        if let Some(verifier) = params.code_verifier {
            form_data.push(("code_verifier", verifier));
        }
        self.get_user_from_form(form_data).await
    }

    async fn get_user_from_token(
        &self,
        access_token: &str,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
        let user_res = self.http_client.get("https://graph.facebook.com/v19.0/me?fields=id,name,email,picture.width(500).height(500)")
            .bearer_auth(access_token)
            .send().await?.error_for_status()?
            .json::<Value>()
            .await?;

        let avatar = user_res["picture"]["data"]["url"]
            .as_str()
            .map(String::from);

        Ok(ConnectUser {
            id: user_res["id"].as_str().map(String::from).ok_or_else(|| {
                crate::error::ConnectError::Provider("Missing user id".to_string())
            })?,
            name: user_res["name"]
                .as_str()
                .map(String::from)
                .unwrap_or_default(),
            email: user_res["email"].as_str().map(String::from),
            avatar_url: avatar,
            email_verified: None,
            raw_data: user_res,
            access_token: secrecy::SecretString::from(access_token.to_string()),
            refresh_token: None,
            expires_in: None,
        })
    }

    fn token_url(&self) -> String {
        "https://graph.facebook.com/v19.0/oauth/access_token".to_string()
    }

    crate::impl_standard_refresh_token!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{HttpClient, HttpRequest, HttpResponse};
    use serde_json::json;
    use std::sync::Arc;

    struct MockFacebookClient {
        token_status: u16,
        token_body: serde_json::Value,
        user_status: u16,
        user_body: serde_json::Value,
    }

    #[async_trait]
    impl HttpClient for MockFacebookClient {
        async fn execute(
            &self,
            req: HttpRequest,
        ) -> Result<HttpResponse, crate::error::ConnectError> {
            if req.url.contains("access_token") {
                Ok(HttpResponse {
                    status: self.token_status,
                    body: self.token_body.clone(),
                })
            } else if req.url.contains("graph.facebook.com/v19.0/me") {
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
    async fn test_facebook_get_user_success() {
        let provider = FacebookProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect.url".to_string(),
        )
        .with_http_client(Arc::new(MockFacebookClient {
            token_status: 200,
            token_body: json!({
                "access_token": "mock_access_token",
                "expires_in": 3600
            }),
            user_status: 200,
            user_body: json!({
                "id": "12345",
                "name": "Test User",
                "email": "test@example.com",
                "picture": {
                    "data": {
                        "url": "https://avatar.url"
                    }
                }
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
        assert_eq!(user.avatar_url.as_deref(), Some("https://avatar.url"));
    }

    #[tokio::test]
    async fn test_facebook_token_error() {
        let provider = FacebookProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect.url".to_string(),
        )
        .with_http_client(Arc::new(MockFacebookClient {
            token_status: 400,
            token_body: json!({"error": {"message": "invalid_grant"}}),
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
    async fn test_facebook_missing_id() {
        let provider = FacebookProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect.url".to_string(),
        )
        .with_http_client(Arc::new(MockFacebookClient {
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
    async fn test_facebook_refresh_token_success() {
        let provider = FacebookProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect.url".to_string(),
        )
        .with_http_client(Arc::new(MockFacebookClient {
            token_status: 200,
            token_body: json!({
                "access_token": "new_access_token",
                "refresh_token": "new_refresh_token",
                "expires_in": 3600
            }),
            user_status: 200,
            user_body: json!({
                "id": "12345",
                "name": "Test User Refreshed",
                "email": "test@example.com",
                "picture": {
                    "data": {
                        "url": "https://avatar.url"
                    }
                }
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
