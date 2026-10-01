use super::*;
use crate::provider::Provider;

#[test]
fn test_redirect_url() {
    let provider = Auth0Provider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("client_secret".to_string()),
        "https://redirect.url".to_string(),
        "test.auth0.com".to_string(),
    );
    let url = provider.redirect_url();
    assert!(url.starts_with("https://test.auth0.com/authorize?"));
    assert!(url.contains("client_id=client_id"));
    assert!(url.contains("redirect_uri=https%3A%2F%2Fredirect.url"));
}

#[tokio::test]
async fn test_auth0_empty_credentials_select_offline_mock() {
    let provider = Auth0Provider::try_new(
        "",
        secrecy::SecretString::from("".to_string()),
        "https://app.example/callback",
        "tenant.auth0.com",
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

#[test]
fn test_redirect_url_invalid_domain() {
    let result = Auth0Provider::try_new(
        "client_id",
        secrecy::SecretString::from("client_secret".to_string()),
        "https://redirect.url",
        "invalid domain",
    );
    assert!(matches!(
        result,
        Err(ConnectError::InvalidConfiguration {
            field: "domain",
            ..
        })
    ));
}

use crate::client::{HttpClient, HttpRequest, HttpResponse};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

struct MockAuth0Client {
    token_status: u16,
    token_body: serde_json::Value,
    user_status: u16,
    user_body: serde_json::Value,
}

#[async_trait]
impl HttpClient for MockAuth0Client {
    async fn execute(&self, req: HttpRequest) -> Result<HttpResponse, crate::error::ConnectError> {
        if req.url.contains("token") {
            Ok(HttpResponse {
                status: self.token_status,
                body: self.token_body.clone(),
            })
        } else if req.url.contains("userinfo") {
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
async fn test_auth0_get_user_success() {
    let provider = Auth0Provider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("client_secret".to_string()),
        "https://redirect.url".to_string(),
        "test.auth0.com".to_string(),
    )
    .with_http_client(Arc::new(MockAuth0Client {
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
            "picture": "https://avatar.url",
            "email_verified": true
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
async fn test_auth0_token_error() {
    let provider = Auth0Provider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("client_secret".to_string()),
        "https://redirect.url".to_string(),
        "test.auth0.com".to_string(),
    )
    .with_http_client(Arc::new(MockAuth0Client {
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
async fn test_auth0_missing_id() {
    let provider = Auth0Provider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("client_secret".to_string()),
        "https://redirect.url".to_string(),
        "test.auth0.com".to_string(),
    )
    .with_http_client(Arc::new(MockAuth0Client {
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
async fn test_auth0_refresh_token_success() {
    let provider = Auth0Provider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("client_secret".to_string()),
        "https://redirect.url".to_string(),
        "test.auth0.com".to_string(),
    )
    .with_http_client(Arc::new(MockAuth0Client {
        token_status: 200,
        token_body: json!({
            "access_token": "new_access_token",
            "refresh_token": "new_refresh_token",
            "expires_in": 3600
        }),
        user_status: 200,
        user_body: json!({
            "sub": "user_123",
            "name": "Test User Refreshed",
            "email": "test@example.com",
            "picture": "https://avatar.url",
            "email_verified": true
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

#[test]
#[cfg(feature = "retry")]
fn test_auth0_with_retry() {
    let provider = Auth0Provider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("client_secret".to_string()),
        "https://redirect.url".to_string(),
        "test.auth0.com".to_string(),
    );
    let original_client = provider.http_client.clone();
    let provider = provider.with_retry(3);
    assert_eq!(provider.client_id, "client_id");
    assert!(!std::sync::Arc::ptr_eq(
        &provider.http_client,
        &original_client
    ));
    assert!(
        !std::sync::Arc::ptr_eq(&provider.http_client, &crate::client::DEFAULT_HTTP_CLIENT),
        "with_retry must create a new client, not reuse DEFAULT_HTTP_CLIENT"
    );
}
