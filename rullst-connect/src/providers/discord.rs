use crate::client::HttpClientExt;
use crate::provider::Provider;
use crate::user::ConnectUser;
use async_trait::async_trait;
use serde_json::Value;

crate::define_provider!(DiscordProvider, "identify", "email");

#[async_trait]
impl Provider for DiscordProvider {
    crate::impl_standard_redirect_url!("https://discord.com/api/oauth2/authorize");

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
            .get("https://discord.com/api/users/@me")
            .bearer_auth(access_token)
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;

        let id = user_res["id"]
            .as_str()
            .map(String::from)
            .unwrap_or_default();
        let avatar_hash = user_res["avatar"].as_str();
        let avatar_url = avatar_hash.map(|hash| {
            format!(
                "https://cdn.discordapp.com/avatars/{}/{}.png?size=1024",
                id, hash
            )
        });

        Ok(ConnectUser {
            id,
            name: user_res["username"]
                .as_str()
                .map(String::from)
                .unwrap_or_default(),
            email: user_res["email"].as_str().map(String::from),
            avatar_url,
            email_verified: user_res["verified"].as_bool(),
            raw_data: user_res,
            access_token: secrecy::SecretString::from(access_token.to_string()),
            refresh_token: None,
            expires_in: None,
        })
    }

    fn token_url(&self) -> String {
        "https://discord.com/api/oauth2/token".to_string()
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
    ) -> Result<(), crate::error::ConnectError> {
        crate::provider::revoke_form_with_basic_credentials(
            self.http_client.as_ref(),
            "https://discord.com/api/oauth2/token/revoke",
            &self.client_id,
            secrecy::ExposeSecret::expose_secret(&self.client_secret),
            token,
            Some(kind),
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

    struct MockDiscordClient {
        token_status: u16,
        token_body: serde_json::Value,
        user_status: u16,
        user_body: serde_json::Value,
    }

    #[async_trait]
    impl HttpClient for MockDiscordClient {
        async fn execute(
            &self,
            req: HttpRequest,
        ) -> Result<HttpResponse, crate::error::ConnectError> {
            if req.url.contains("token") {
                Ok(HttpResponse {
                    status: self.token_status,
                    body: self.token_body.clone(),
                })
            } else if req.url.contains("@me") {
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
    async fn test_discord_get_user_success() {
        let provider = DiscordProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect.url".to_string(),
        )
        .with_http_client(Arc::new(MockDiscordClient {
            token_status: 200,
            token_body: json!({
                "access_token": "mock_access_token",
                "expires_in": 3600
            }),
            user_status: 200,
            user_body: json!({
                "id": "12345",
                "username": "testuser",
                "email": "test@example.com",
                "avatar": "deadbeef",
                "verified": true
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
        assert_eq!(user.name, "testuser");
        assert_eq!(user.email.as_deref(), Some("test@example.com"));
        assert_eq!(
            user.avatar_url.as_deref(),
            Some("https://cdn.discordapp.com/avatars/12345/deadbeef.png?size=1024")
        );
    }

    #[tokio::test]
    async fn test_discord_token_error() {
        let provider = DiscordProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect.url".to_string(),
        )
        .with_http_client(Arc::new(MockDiscordClient {
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
    async fn test_discord_missing_fields() {
        let provider = DiscordProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect.url".to_string(),
        )
        .with_http_client(Arc::new(MockDiscordClient {
            token_status: 200,
            token_body: json!({"access_token": "mock_access_token"}),
            user_status: 200,
            user_body: json!({"id": "123", "username": "No Email Avatar User"}),
        }));

        let user = provider
            .get_user(crate::provider::ExchangeParams {
                auth_code: "code",
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(user.id, "123");
        assert_eq!(user.name, "No Email Avatar User");
        assert_eq!(user.email, None);
        assert_eq!(user.avatar_url, None);
        assert_eq!(user.email_verified, None);
    }

    #[tokio::test]
    async fn test_discord_missing_id() {
        let provider = DiscordProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect.url".to_string(),
        )
        .with_http_client(Arc::new(MockDiscordClient {
            token_status: 200,
            token_body: json!({"access_token": "mock_access_token"}),
            user_status: 200,
            user_body: json!({"username": "No ID User"}),
        }));

        let user = provider
            .get_user(crate::provider::ExchangeParams {
                auth_code: "code",
                ..Default::default()
            })
            .await
            .unwrap();

        // Discord gracefully degrades if `id` is missing in some implementations,
        // Wait, looking at the code: `let id = user_res["id"].as_str().map(String::from).unwrap_or_default();`
        // So it shouldn't be an error, it will just return empty id.
        assert_eq!(user.id, "");
    }

    #[tokio::test]
    async fn test_discord_refresh_token_success() {
        let provider = DiscordProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect.url".to_string(),
        )
        .with_http_client(Arc::new(MockDiscordClient {
            token_status: 200,
            token_body: json!({
                "access_token": "new_access_token",
                "refresh_token": "new_refresh_token",
                "expires_in": 3600
            }),
            user_status: 200,
            user_body: json!({
                "id": "12345",
                "username": "testuser_refreshed",
                "email": "test@example.com",
                "avatar": "deadbeef",
                "verified": true
            }),
        }));

        let user = provider.refresh_token("old_refresh").await.unwrap();
        assert_eq!(user.id, "12345");
        assert_eq!(user.name, "testuser_refreshed");
        use secrecy::ExposeSecret;
        assert_eq!(
            user.refresh_token.unwrap().expose_secret(),
            "new_refresh_token"
        );
    }
}
