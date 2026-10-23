//! Unit tests for the GitHub OAuth provider.
#![allow(clippy::unwrap_used, clippy::expect_used)]
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
    async fn execute(&self, req: HttpRequest) -> Result<HttpResponse, crate::error::ConnectError> {
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
    assert!(matches!(
        err,
        crate::error::ConnectError::ProviderApiError { code, message }
            if code == "authorization_pending" && message == "User has not yet entered code."
    ));
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

/// Answers the token, `/user` and `/user/emails` requests and records the
/// URL and bearer token of each GET.
struct EmailsClient {
    emails_status: u16,
    emails: serde_json::Value,
    gets: std::sync::Mutex<Vec<(String, Option<String>)>>,
}

#[async_trait]
impl HttpClient for EmailsClient {
    async fn execute(&self, req: HttpRequest) -> Result<HttpResponse, crate::error::ConnectError> {
        if req.url.contains("access_token") {
            return Ok(HttpResponse {
                status: 200,
                body: json!({"access_token": "gho_token", "token_type": "bearer"}),
            });
        }
        self.gets
            .lock()
            .unwrap()
            .push((req.url.clone(), req.bearer_auth.clone()));
        if req.url.ends_with("/user/emails") {
            return Ok(HttpResponse {
                status: self.emails_status,
                body: self.emails.clone(),
            });
        }
        Ok(HttpResponse {
            status: 200,
            body: json!({
                "id": 583231,
                "login": "octocat",
                "name": "The Octocat",
                "email": "public@example.test"
            }),
        })
    }
}

async fn signed_in(
    emails_status: u16,
    emails: serde_json::Value,
) -> (ConnectUser, Vec<(String, Option<String>)>) {
    let client = Arc::new(EmailsClient {
        emails_status,
        emails,
        gets: std::sync::Mutex::new(Vec::new()),
    });
    let provider = GithubProvider::try_new(
        "client_id",
        secrecy::SecretString::from("client_secret".to_string()),
        "https://app.example/auth/github/callback",
    )
    .unwrap()
    .with_http_client(client.clone());
    let user = provider
        .get_user(crate::provider::ExchangeParams {
            auth_code: "code",
            ..Default::default()
        })
        .await
        .unwrap();
    let gets = client.gets.lock().unwrap().clone();
    (user, gets)
}

#[tokio::test]
async fn the_subject_is_the_numeric_id_and_the_email_the_verified_primary() {
    let (user, gets) = signed_in(
        200,
        json!([
            {"email": "other@example.test", "primary": false, "verified": true},
            {"email": "primary@example.test", "primary": true, "verified": true}
        ]),
    )
    .await;
    assert_eq!(user.id, "583231");
    assert_eq!(user.name, "The Octocat");
    assert_eq!(user.email.as_deref(), Some("primary@example.test"));
    assert_eq!(user.email_verified, Some(true));
    assert_eq!(
        gets,
        [
            (
                "https://api.github.com/user".to_owned(),
                Some("gho_token".to_owned())
            ),
            (
                "https://api.github.com/user/emails".to_owned(),
                Some("gho_token".to_owned())
            ),
        ]
    );
}

#[tokio::test]
async fn an_unverified_primary_is_reported_unverified() {
    // A verified secondary address never stands in for the primary one.
    let (user, _) = signed_in(
        200,
        json!([
            {"email": "verified@example.test", "primary": false, "verified": true},
            {"email": "primary@example.test", "primary": true, "verified": false}
        ]),
    )
    .await;
    assert_eq!(user.email.as_deref(), Some("primary@example.test"));
    assert_eq!(user.email_verified, Some(false));

    let (user, _) = signed_in(
        200,
        json!([{"email": "primary@example.test", "primary": true}]),
    )
    .await;
    assert_eq!(user.email_verified, Some(false));
}

#[tokio::test]
async fn without_a_primary_address_the_public_email_stays_unverified() {
    for (status, emails) in [
        (
            200,
            json!([{"email": "a@example.test", "primary": false, "verified": true}]),
        ),
        (200, json!({"message": "not a list"})),
        (200, json!([{"primary": true, "verified": true}])),
        (
            403,
            json!({"message": "Resource not accessible by integration"}),
        ),
    ] {
        let (user, _) = signed_in(status, emails).await;
        assert_eq!(user.id, "583231");
        assert_eq!(user.email.as_deref(), Some("public@example.test"));
        assert_eq!(user.email_verified, None);
    }
}

#[tokio::test]
async fn mock_credentials_sign_in_a_verified_primary_address_offline() {
    let provider = GithubProvider::try_new(
        "mock_github",
        secrecy::SecretString::from("mock_github".to_string()),
        "http://localhost:3000/auth/github/callback",
    )
    .unwrap();
    let user = provider
        .get_user(crate::provider::ExchangeParams {
            auth_code: "mock_authorization_code",
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(user.id, "1");
    assert_eq!(user.email.as_deref(), Some("mock@example.invalid"));
    assert_eq!(user.email_verified, Some(true));
}
