use crate::client::HttpClientExt;
use crate::provider::Provider;
use crate::user::ConnectUser;
use async_trait::async_trait;
use serde_json::Value;

crate::define_provider!(GithubProvider, "user:email");

impl GithubProvider {
    async fn get_user_from_form(
        &self,
        form_data: &crate::provider::TokenExchangeForm<'_>,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
        let token_res = self
            .http_client
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .form(form_data)
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;

        if let Some(err) = token_res["error"].as_str() {
            let err_desc = token_res["error_description"].as_str().unwrap_or("");
            return Err(crate::error::ConnectError::Token(format!(
                "Provider returned error: {} - {}",
                err, err_desc
            )));
        }

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
impl Provider for GithubProvider {
    crate::impl_standard_redirect_url!("https://github.com/login/oauth/authorize");

    async fn get_user(
        &self,
        params: crate::provider::ExchangeParams<'_>,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
        crate::provider::require_oauth_only(params.expected_nonce)?;
        let form_data = crate::provider::TokenExchangeForm {
            client_id: self.client_id.as_str(),
            client_secret: Some(secrecy::ExposeSecret::expose_secret(&self.client_secret)),
            code: params.auth_code,
            grant_type: None,
            redirect_uri: self.redirect_url.as_str(),
            code_verifier: params.code_verifier,
        };
        self.get_user_from_form(&form_data).await
    }

    async fn get_user_from_token(
        &self,
        access_token: &str,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
        let user_res = self
            .http_client
            .get("https://api.github.com/user")
            .bearer_auth(access_token)
            .header("User-Agent", "rullst-connect")
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;

        Ok(ConnectUser {
            id: user_res["id"]
                .as_i64()
                .ok_or_else(|| crate::error::ConnectError::Provider("Missing id".to_string()))?
                .to_string(),
            name: user_res["name"]
                .as_str()
                .unwrap_or(user_res["login"].as_str().unwrap_or(""))
                .to_string(),
            email: user_res["email"].as_str().map(String::from),
            avatar_url: user_res["avatar_url"].as_str().map(String::from),
            email_verified: None,
            raw_data: user_res,
            access_token: secrecy::SecretString::from(access_token.to_string()),
            refresh_token: None,
            expires_in: None,
        })
    }

    fn token_url(&self) -> String {
        "https://github.com/login/oauth/access_token".to_string()
    }

    crate::impl_standard_refresh_token!();

    async fn revoke_token_with_kind(
        &self,
        token: &str,
        kind: crate::provider::RevocationTokenKind,
    ) -> Result<(), crate::error::ConnectError> {
        if kind != crate::provider::RevocationTokenKind::AccessToken {
            return Err(crate::provider::unsupported_revocation_kind("GitHub", kind));
        }
        let endpoint = crate::provider::provider_url_with_segments(
            "https://api.github.com/",
            &["applications", &self.client_id, "token"],
        )?;
        crate::provider::revoke_json_with_basic_delete(
            self.http_client.as_ref(),
            endpoint,
            &self.client_id,
            secrecy::ExposeSecret::expose_secret(&self.client_secret),
            token,
        )
        .await
    }

    async fn request_device_code(
        &self,
    ) -> Result<crate::user::DeviceAuthorizationResponse, crate::error::ConnectError> {
        let mut form = vec![("client_id", self.client_id.as_str())];
        if !self.scopes.is_empty() {
            form.push(("scope", self.scopes.as_str()));
        }

        let res = self
            .http_client
            .post("https://github.com/login/device/code")
            .header("Accept", "application/json")
            .form(&form)
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;

        let device_code = res["device_code"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| {
                crate::error::ConnectError::Provider(
                    "Missing device_code from Github response".to_string(),
                )
            })?;
        let user_code = res["user_code"].as_str().map(String::from).ok_or_else(|| {
            crate::error::ConnectError::Provider(
                "Missing user_code from Github response".to_string(),
            )
        })?;
        let verification_uri = res["verification_uri"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| {
                crate::error::ConnectError::Provider(
                    "Missing verification_uri from Github response".to_string(),
                )
            })?;

        Ok(crate::user::DeviceAuthorizationResponse {
            device_code,
            user_code,
            verification_uri,
            verification_uri_complete: res["verification_uri_complete"].as_str().map(String::from),
            expires_in: res["expires_in"].as_u64().unwrap_or(900),
            interval: res["interval"].as_u64(),
        })
    }

    async fn poll_device_token(
        &self,
        device_code: &str,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
        let token_res = self
            .http_client
            .post(self.token_url())
            .header("Accept", "application/json")
            .form(&[
                ("client_id", self.client_id.as_str()),
                ("device_code", device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;

        if let Some(err) = token_res["error"].as_str() {
            let err_desc = token_res["error_description"].as_str().unwrap_or("");
            return Err(crate::error::ConnectError::Token(format!(
                "Provider returned error: {} - {}",
                err, err_desc
            )));
        }

        let access_token = token_res["access_token"].as_str().ok_or_else(|| {
            crate::error::ConnectError::Token(
                "Failed to get access_token during device poll. (Authorization pending?)"
                    .to_string(),
            )
        })?;

        let mut user = self.get_user_from_token(access_token).await?;
        user.refresh_token = token_res["refresh_token"]
            .as_str()
            .map(|s| secrecy::SecretString::from(s.to_string()));
        user.expires_in = crate::provider::token_lifetime(&token_res)?;
        Ok(user)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{HttpClient, HttpRequest, HttpResponse};
    use serde_json::json;
    use std::sync::Arc;

    struct MockGithubClient {
        token_status: u16,
        token_body: serde_json::Value,
        user_status: u16,
        user_body: serde_json::Value,
        device_status: u16,
        device_body: serde_json::Value,
    }

    #[async_trait]
    impl HttpClient for MockGithubClient {
        async fn execute(
            &self,
            req: HttpRequest,
        ) -> Result<HttpResponse, crate::error::ConnectError> {
            if req.url.contains("access_token") {
                Ok(HttpResponse {
                    status: self.token_status,
                    body: self.token_body.clone(),
                })
            } else if req.url.contains("device/code") {
                Ok(HttpResponse {
                    status: self.device_status,
                    body: self.device_body.clone(),
                })
            } else {
                Ok(HttpResponse {
                    status: self.user_status,
                    body: self.user_body.clone(),
                })
            }
        }
    }

    #[tokio::test]
    async fn test_github_get_user() {
        let provider = GithubProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect.url".to_string(),
        )
        .with_http_client(Arc::new(MockGithubClient {
            token_status: 200,
            token_body: json!({
                "access_token": "mock_token",
                "token_type": "bearer",
                "scope": "user:email"
            }),
            user_status: 200,
            user_body: json!({
                "id": 1234567,
                "login": "octocat",
                "name": "monalisa octocat",
                "email": "octocat@github.com",
                "avatar_url": "https://github.com/images/error/octocat_happy.gif"
            }),
            device_status: 200,
            device_body: json!({}),
        }));

        let params = crate::provider::ExchangeParams {
            auth_code: "mock_code",
            code_verifier: None,
            expected_nonce: None,
        };
        let user = provider.get_user(params).await.expect("Failed to get user");
        assert_eq!(user.id, "1234567");
        assert_eq!(user.name, "monalisa octocat");
        assert_eq!(user.email, Some("octocat@github.com".to_string()));
        assert_eq!(
            user.avatar_url,
            Some("https://github.com/images/error/octocat_happy.gif".to_string())
        );
    }

    #[tokio::test]
    async fn test_github_token_error() {
        let provider = GithubProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect.url".to_string(),
        )
        .with_http_client(Arc::new(MockGithubClient {
            token_status: 400,
            token_body: json!({"error": "invalid_grant"}),
            user_status: 200,
            user_body: json!({}),
            device_status: 200,
            device_body: json!({}),
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
    async fn test_github_request_device_code_success() {
        let provider = GithubProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect.url".to_string(),
        )
        .with_http_client(Arc::new(MockGithubClient {
            token_status: 200,
            token_body: json!({}),
            user_status: 200,
            user_body: json!({}),
            device_status: 200,
            device_body: json!({
                "device_code": "device_123",
                "user_code": "user_456",
                "verification_uri": "https://github.com/login/device",
                "expires_in": 900,
                "interval": 5
            }),
        }));

        let res = provider.request_device_code().await.unwrap();
        assert_eq!(res.device_code, "device_123");
        assert_eq!(res.user_code, "user_456");
        assert_eq!(res.verification_uri, "https://github.com/login/device");
    }

    #[tokio::test]
    async fn test_github_poll_device_token_success() {
        let provider = GithubProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect.url".to_string(),
        )
        .with_http_client(Arc::new(MockGithubClient {
            token_status: 200,
            token_body: json!({
                "access_token": "mock_token",
                "token_type": "bearer",
                "scope": "user:email"
            }),
            user_status: 200,
            user_body: json!({
                "id": 1234567,
                "login": "octocat",
                "name": "monalisa octocat",
                "email": "octocat@github.com",
                "avatar_url": "https://github.com/images/error/octocat_happy.gif"
            }),
            device_status: 200,
            device_body: json!({}),
        }));

        let user = provider.poll_device_token("device_123").await.unwrap();
        assert_eq!(user.id, "1234567");
    }

    #[tokio::test]
    async fn test_github_poll_device_token_error() {
        let provider = GithubProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect.url".to_string(),
        )
        .with_http_client(Arc::new(MockGithubClient {
            token_status: 200, // Github returns 200 for authorization_pending
            token_body: json!({
                "error": "authorization_pending",
                "error_description": "User has not yet entered code."
            }),
            user_status: 200,
            user_body: json!({}),
            device_status: 200,
            device_body: json!({}),
        }));

        let err = provider.poll_device_token("device_123").await.unwrap_err();
        assert!(
            matches!(err, crate::error::ConnectError::Token(msg) if msg.contains("authorization_pending"))
        );
    }

    #[tokio::test]
    async fn test_github_poll_device_token_missing_token() {
        let provider = GithubProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect.url".to_string(),
        )
        .with_http_client(Arc::new(MockGithubClient {
            token_status: 200,
            token_body: json!({}), // No access_token and no error
            user_status: 200,
            user_body: json!({}),
            device_status: 200,
            device_body: json!({}),
        }));

        let err = provider.poll_device_token("device_123").await.unwrap_err();
        assert!(
            matches!(err, crate::error::ConnectError::Token(msg) if msg.contains("Failed to get access_token during device poll"))
        );
    }

    #[tokio::test]
    async fn test_github_request_device_code_scopes() {
        struct MockScopeClient {
            expect_scope: bool,
        }

        #[async_trait]
        impl HttpClient for MockScopeClient {
            async fn execute(
                &self,
                req: HttpRequest,
            ) -> Result<HttpResponse, crate::error::ConnectError> {
                if req.url.contains("device/code") {
                    let body_str = req.form.unwrap_or_default();
                    if self.expect_scope {
                        assert!(body_str.contains("scope="));
                    } else {
                        assert!(!body_str.contains("scope="));
                    }
                    Ok(HttpResponse {
                        status: 200,
                        body: serde_json::json!({
                            "device_code": "device_123",
                            "user_code": "USER123",
                            "verification_uri": "https://github.com/login/device",
                            "expires_in": 900,
                            "interval": 5
                        }),
                    })
                } else {
                    Err(crate::error::ConnectError::Provider(
                        "Unexpected URL".to_string(),
                    ))
                }
            }
        }

        let provider_with_scope = GithubProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect.url".to_string(),
        )
        .with_scopes(&["read:user"])
        .with_http_client(Arc::new(MockScopeClient { expect_scope: true }));
        provider_with_scope.request_device_code().await.unwrap();

        let provider_no_scope = GithubProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect.url".to_string(),
        )
        .with_scopes(&[])
        .with_http_client(Arc::new(MockScopeClient {
            expect_scope: false,
        }));
        provider_no_scope.request_device_code().await.unwrap();
    }
}
