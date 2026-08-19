//! Unit tests for provider traits, parameters, token helpers, and JWKS caching.

use super::*;
use crate::error::ConnectError;
use crate::user::ConnectUser;
use async_trait::async_trait;

struct DummyProvider {
    base_url: String,
}

#[async_trait]
impl Provider for DummyProvider {
    fn redirect_url(&self) -> String {
        self.base_url.clone()
    }

    fn token_url(&self) -> String {
        "".to_string()
    }

    async fn get_user(&self, _params: ExchangeParams<'_>) -> Result<ConnectUser, ConnectError> {
        self.get_user_from_token("dummy_access_token").await
    }

    async fn get_user_from_token(
        &self,
        access_token: &str,
    ) -> Result<ConnectUser, ConnectError> {
        Ok(ConnectUser {
            id: "dummy_id".into(),
            name: "Dummy User".into(),
            email: Some("dummy@example.com".into()),
            email_verified: Some(true),
            avatar_url: None,
            raw_data: serde_json::json!({}),
            access_token: secrecy::SecretString::from(access_token.to_string()),
            refresh_token: None,
            expires_in: None,
        })
    }
}

#[test]
fn test_redirect_url_with_state() {
    let provider_no_query = DummyProvider {
        base_url: "https://example.com/auth".to_string(),
    };
    assert_eq!(
        provider_no_query.redirect_url_with_state("my_state"),
        "https://example.com/auth?state=my_state"
    );

    let provider_with_query = DummyProvider {
        base_url: "https://example.com/auth?client_id=123".to_string(),
    };
    assert_eq!(
        provider_with_query.redirect_url_with_state("my_state"),
        "https://example.com/auth?client_id=123&state=my_state"
    );
}

#[test]
fn test_redirect_url_with_pkce() {
    let provider_no_query = DummyProvider {
        base_url: "https://example.com/auth".to_string(),
    };
    assert_eq!(
        provider_no_query.redirect_url_with_pkce("my_challenge"),
        "https://example.com/auth?code_challenge=my_challenge&code_challenge_method=S256"
    );

    let provider_with_query = DummyProvider {
        base_url: "https://example.com/auth?client_id=123".to_string(),
    };
    assert_eq!(
        provider_with_query.redirect_url_with_pkce("my_challenge"),
        "https://example.com/auth?client_id=123&code_challenge=my_challenge&code_challenge_method=S256"
    );
}

#[test]
fn test_redirect_url_with_pkce_and_state() {
    let provider_no_query = DummyProvider {
        base_url: "https://example.com/auth".to_string(),
    };
    assert_eq!(
        provider_no_query.redirect_url_with_pkce_and_state("my_challenge", "my_state"),
        "https://example.com/auth?code_challenge=my_challenge&code_challenge_method=S256&state=my_state"
    );

    let provider_with_query = DummyProvider {
        base_url: "https://example.com/auth?client_id=123".to_string(),
    };
    assert_eq!(
        provider_with_query.redirect_url_with_pkce_and_state("my_challenge", "my_state"),
        "https://example.com/auth?client_id=123&code_challenge=my_challenge&code_challenge_method=S256&state=my_state"
    );
}

#[tokio::test]
async fn test_default_revoke_token() {
    let provider = DummyProvider {
        base_url: "".to_string(),
    };
    let result = provider.revoke_token("some_token").await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ConnectError::Token(msg) => {
            assert_eq!(msg, "Token revocation is not supported by this provider");
        }
        _ => panic!("Expected ConnectError::Token"),
    }
}

#[tokio::test]
async fn test_default_poll_device_token() {
    let provider = DummyProvider {
        base_url: "".to_string(),
    };
    let result = provider.poll_device_token("some_code").await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ConnectError::Provider(msg) => {
            assert_eq!(
                msg,
                "Device Authorization is not supported by this provider"
            );
        }
        _ => panic!("Expected ConnectError::Provider"),
    }
}

#[tokio::test]
async fn test_default_request_device_code() {
    let provider = DummyProvider {
        base_url: "".to_string(),
    };
    let result = provider.request_device_code().await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ConnectError::Provider(msg) => {
            assert_eq!(
                msg,
                "Device Authorization is not supported by this provider"
            );
        }
        _ => panic!("Expected ConnectError::Provider"),
    }
}

#[tokio::test]
async fn test_default_refresh_token() {
    let provider = DummyProvider {
        base_url: "".to_string(),
    };
    let result = provider.refresh_token("some_token").await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ConnectError::Token(msg) => {
            assert_eq!(msg, "Refresh token is not supported by this provider");
        }
        _ => panic!("Expected ConnectError::Token"),
    }
}

#[test]
fn test_redirect_url_with_pkce_and_state_multiple_query_params() {
    let provider_multiple_query = DummyProvider {
        base_url: "https://example.com/auth?foo=bar&baz=qux".to_string(),
    };
    assert_eq!(
        provider_multiple_query.redirect_url_with_pkce_and_state("my_challenge", "my_state"),
        "https://example.com/auth?foo=bar&baz=qux&code_challenge=my_challenge&code_challenge_method=S256&state=my_state"
    );
}

#[test]
fn test_build_oauth_params_variations() {
    // 1. Empty scopes
    let mut serializer = build_oauth_params("", "client", "redirect", "", None, None);
    let query = serializer.finish();
    assert!(query.contains("client_id=client"));
    assert!(query.contains("redirect_uri=redirect"));
    assert!(!query.contains("scope"));

    // 2. Single scope
    let mut serializer = build_oauth_params("", "client", "redirect", "read", None, None);
    let query = serializer.finish();
    assert!(query.contains("scope=read"));

    // 3. Multiple scopes
    let mut serializer = build_oauth_params(
        "",
        "client",
        "redirect",
        "read write",
        Some("state123"),
        Some("pkce_challenge"),
    );
    let query = serializer.finish();
    assert!(query.contains("scope=read+write"));
    assert!(query.contains("state=state123"));
    assert!(query.contains("code_challenge=pkce_challenge"));
    assert!(query.contains("code_challenge_method=S256"));
}

struct MockFetchClient;
#[async_trait]
impl crate::client::HttpClient for MockFetchClient {
    async fn execute(
        &self,
        req: crate::client::HttpRequest,
    ) -> Result<crate::client::HttpResponse, crate::error::ConnectError> {
        if req.url.contains("error") {
            Ok(crate::client::HttpResponse {
                status: 200,
                body: serde_json::json!({
                    "error": "invalid_request",
                    "error_description": "Test error"
                }),
            })
        } else {
            Ok(crate::client::HttpResponse {
                status: 200,
                body: serde_json::json!({
                    "access_token": "mock_access",
                    "refresh_token": "mock_refresh",
                    "expires_in": 3600
                }),
            })
        }
    }
}

#[tokio::test]
async fn test_fetch_access_token() {
    let client = MockFetchClient;
    let form = TokenExchangeForm {
        client_id: "client_id",
        client_secret: Some("client_secret"),
        code: "auth_code",
        grant_type: Some("authorization_code"),
        redirect_uri: "https://redirect",
        code_verifier: Some("verifier"),
    };
    let res = fetch_access_token(&client, "https://example.com/token", &form)
        .await
        .expect("Failed to fetch access token");

    assert_eq!(res.access_token, "mock_access");
    assert_eq!(res.refresh_token.as_deref(), Some("mock_refresh"));
    assert_eq!(res.expires_in, Some(3600));
}

#[tokio::test]
async fn test_fetch_access_token_error() {
    let client = MockFetchClient;
    let form = TokenExchangeForm {
        client_id: "client_id",
        client_secret: Some("client_secret"),
        code: "auth_code",
        grant_type: Some("authorization_code"),
        redirect_uri: "https://redirect",
        code_verifier: Some("verifier"),
    };
    // Use a URL with "error" so the mock returns the error JSON.
    let err = fetch_access_token(&client, "https://example.com/error", &form)
        .await
        .unwrap_err();

    match err {
        ConnectError::Token(msg) => {
            assert!(msg.contains("invalid_request"));
            assert!(msg.contains("Test error"));
        }
        _ => panic!("Expected ConnectError::Token"),
    }
}

#[tokio::test]
async fn test_fetch_refresh_token() {
    let client = MockFetchClient;
    let res = fetch_refresh_token(
        &client,
        "https://example.com/token",
        "client_id",
        "client_secret",
        "mock_refresh",
    )
    .await
    .expect("Failed to fetch refresh token");

    assert_eq!(res.access_token, "mock_access");
    assert_eq!(res.refresh_token.as_deref(), Some("mock_refresh"));
    assert_eq!(res.expires_in, Some(3600));
}

#[tokio::test]
async fn test_fetch_refresh_token_error() {
    let client = MockFetchClient;
    let err = fetch_refresh_token(
        &client,
        "https://example.com/error",
        "client_id",
        "client_secret",
        "mock_refresh",
    )
    .await
    .unwrap_err();

    match err {
        ConnectError::Token(msg) => {
            assert!(msg.contains("invalid_request"));
            assert!(msg.contains("Test error"));
        }
        _ => panic!("Expected ConnectError::Token"),
    }
}

struct MockFetchClientMissingToken;
#[async_trait]
impl crate::client::HttpClient for MockFetchClientMissingToken {
    async fn execute(
        &self,
        _req: crate::client::HttpRequest,
    ) -> Result<crate::client::HttpResponse, crate::error::ConnectError> {
        Ok(crate::client::HttpResponse {
            status: 200,
            body: serde_json::json!({
                "expires_in": 3600
            }),
        })
    }
}

#[tokio::test]
async fn test_fetch_access_token_missing() {
    let client = MockFetchClientMissingToken;
    let form = TokenExchangeForm {
        client_id: "client_id",
        client_secret: Some("client_secret"),
        code: "auth_code",
        grant_type: Some("authorization_code"),
        redirect_uri: "https://redirect",
        code_verifier: Some("verifier"),
    };
    let err = fetch_access_token(&client, "https://example.com/token", &form)
        .await
        .unwrap_err();

    match err {
        ConnectError::Token(msg) => assert_eq!(msg, "Failed to get access_token"),
        _ => panic!("Expected ConnectError::Token"),
    }
}

#[tokio::test]
async fn test_fetch_refresh_token_missing() {
    let client = MockFetchClientMissingToken;
    let err = fetch_refresh_token(
        &client,
        "https://example.com/token",
        "client_id",
        "client_secret",
        "mock_refresh",
    )
    .await
    .unwrap_err();

    match err {
        ConnectError::Token(msg) => {
            assert_eq!(msg, "Failed to get access_token during refresh")
        }
        _ => panic!("Expected ConnectError::Token"),
    }
}

#[tokio::test]
async fn test_exchange_and_get_user() {
    struct MockUserClient;
    #[async_trait]
    impl crate::client::HttpClient for MockUserClient {
        async fn execute(
            &self,
            req: crate::client::HttpRequest,
        ) -> Result<crate::client::HttpResponse, crate::error::ConnectError> {
            if req.url.contains("token") {
                Ok(crate::client::HttpResponse {
                    status: 200,
                    body: serde_json::json!({
                        "access_token": "mock_access",
                        "refresh_token": "mock_refresh",
                        "expires_in": 3600
                    }),
                })
            } else if req.url.contains("user") {
                Ok(crate::client::HttpResponse {
                    status: 200,
                    body: serde_json::json!({
                        "id": "123",
                        "name": "Test User"
                    }),
                })
            } else {
                Err(crate::error::ConnectError::Provider(
                    "Unexpected URL".to_string(),
                ))
            }
        }
    }

    struct SimpleProvider;
    #[async_trait]
    impl Provider for SimpleProvider {
        fn redirect_url(&self) -> String {
            "".into()
        }
        fn token_url(&self) -> String {
            "".into()
        }
        async fn get_user(
            &self,
            _params: ExchangeParams<'_>,
        ) -> Result<ConnectUser, ConnectError> {
            Err(ConnectError::Provider(
                "get_user not implemented for mock".into(),
            ))
        }
        async fn get_user_from_token(
            &self,
            access_token: &str,
        ) -> Result<ConnectUser, ConnectError> {
            Ok(ConnectUser {
                id: "123".into(),
                name: "Test User".into(),
                email: None,
                avatar_url: None,
                email_verified: Some(false),
                raw_data: serde_json::json!({}),
                access_token: secrecy::SecretString::from(access_token.to_string()),
                refresh_token: None,
                expires_in: None,
            })
        }
    }

    let form = TokenExchangeForm {
        client_id: "client",
        client_secret: None,
        code: "code",
        grant_type: None,
        redirect_uri: "redirect",
        code_verifier: None,
    };

    let user = exchange_and_get_user(
        &SimpleProvider,
        &MockUserClient,
        "https://example.com/token",
        &form,
        None,
    )
    .await
    .unwrap();
    assert_eq!(user.id, "123");
    use secrecy::ExposeSecret;
    assert_eq!(user.access_token.expose_secret(), "mock_access");
    assert_eq!(user.refresh_token.unwrap().expose_secret(), "mock_refresh");
    assert_eq!(user.expires_in, Some(3600));
}

#[tokio::test]
async fn test_refresh_and_get_user() {
    struct MockUserClient;
    #[async_trait]
    impl crate::client::HttpClient for MockUserClient {
        async fn execute(
            &self,
            req: crate::client::HttpRequest,
        ) -> Result<crate::client::HttpResponse, crate::error::ConnectError> {
            if req.url.contains("token") {
                Ok(crate::client::HttpResponse {
                    status: 200,
                    body: serde_json::json!({
                        "access_token": "refreshed_access",
                        "refresh_token": "refreshed_refresh",
                        "expires_in": 3600
                    }),
                })
            } else if req.url.contains("user") {
                Ok(crate::client::HttpResponse {
                    status: 200,
                    body: serde_json::json!({
                        "id": "123",
                        "name": "Test User"
                    }),
                })
            } else {
                Err(crate::error::ConnectError::Provider(
                    "Unexpected URL".to_string(),
                ))
            }
        }
    }

    struct SimpleProvider;
    #[async_trait]
    impl Provider for SimpleProvider {
        fn redirect_url(&self) -> String {
            "".into()
        }
        fn token_url(&self) -> String {
            "".into()
        }
        async fn get_user(
            &self,
            _params: ExchangeParams<'_>,
        ) -> Result<ConnectUser, ConnectError> {
            Err(ConnectError::Provider(
                "get_user not implemented for mock".into(),
            ))
        }
        async fn get_user_from_token(
            &self,
            access_token: &str,
        ) -> Result<ConnectUser, ConnectError> {
            Ok(ConnectUser {
                id: "123".into(),
                name: "Test User".into(),
                email: None,
                avatar_url: None,
                email_verified: Some(false),
                raw_data: serde_json::json!({}),
                access_token: secrecy::SecretString::from(access_token.to_string()),
                refresh_token: None,
                expires_in: None,
            })
        }
    }

    let user = refresh_and_get_user(
        &SimpleProvider,
        &MockUserClient,
        "https://example.com/token",
        "client_id",
        &secrecy::SecretString::from("secret".to_string()),
        "old_refresh",
    )
    .await
    .unwrap();
    assert_eq!(user.id, "123");
    use secrecy::ExposeSecret;
    assert_eq!(user.access_token.expose_secret(), "refreshed_access");
    assert_eq!(
        user.refresh_token.unwrap().expose_secret(),
        "refreshed_refresh"
    );
    assert_eq!(user.expires_in, Some(3600));
}

#[tokio::test]
async fn test_exchange_and_get_user_fetch_user_fails() {
    struct MockSuccessTokenClient;
    #[async_trait]
    impl crate::client::HttpClient for MockSuccessTokenClient {
        async fn execute(
            &self,
            _req: crate::client::HttpRequest,
        ) -> Result<crate::client::HttpResponse, crate::error::ConnectError> {
            // Return valid token response
            Ok(crate::client::HttpResponse {
                status: 200,
                body: serde_json::json!({
                    "access_token": "mock_access",
                    "expires_in": 3600
                }),
            })
        }
    }

    struct FailingUserProvider;
    #[async_trait]
    impl Provider for FailingUserProvider {
        fn redirect_url(&self) -> String {
            "".into()
        }
        fn token_url(&self) -> String {
            "".into()
        }
        async fn get_user(
            &self,
            _params: ExchangeParams<'_>,
        ) -> Result<ConnectUser, ConnectError> {
            Err(ConnectError::Provider(
                "get_user not implemented for mock".into(),
            ))
        }
        async fn get_user_from_token(
            &self,
            _access_token: &str,
        ) -> Result<ConnectUser, ConnectError> {
            Err(ConnectError::Provider(
                "Failed to fetch user data".to_string(),
            ))
        }
    }

    let form = TokenExchangeForm {
        client_id: "client",
        client_secret: None,
        code: "code",
        grant_type: None,
        redirect_uri: "uri",
        code_verifier: None,
    };

    let result = exchange_and_get_user(
        &FailingUserProvider,
        &MockSuccessTokenClient,
        "https://example.com/token",
        &form,
        None,
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ConnectError::Provider(msg) => {
            assert_eq!(msg, "Failed to fetch user data");
        }
        _ => panic!("Expected ConnectError::Provider"),
    }
}

#[tokio::test]
async fn test_fetch_and_cache_jwks() {
    struct MockJwksClient;
    #[async_trait]
    impl crate::client::HttpClient for MockJwksClient {
        async fn execute(
            &self,
            req: crate::client::HttpRequest,
        ) -> Result<crate::client::HttpResponse, crate::error::ConnectError> {
            if req.url.contains("jwks") {
                Ok(crate::client::HttpResponse {
                    status: 200,
                    body: serde_json::json!({
                        "keys": [
                            {
                                "kty": "RSA",
                                "kid": "test-kid",
                                "use": "sig",
                                "n": "123",
                                "e": "AQAB"
                            }
                        ]
                    }),
                })
            } else {
                Err(crate::error::ConnectError::Provider("Not found".into()))
            }
        }
    }

    let test_url = "https://example.com/jwks_test";
    {
        let mut cache = JWKS_CACHE.write().await;
        cache.remove(test_url);
    }

    let client = MockJwksClient;
    let jwk_set = fetch_and_cache_jwks(test_url, &client)
        .await
        .expect("Failed to fetch JWKS");
    assert_eq!(jwk_set.keys.len(), 1);

    // Next fetch should be cached (does not require mock client execution if mocked to fail)
    let cached = fetch_and_cache_jwks(test_url, &client).await.unwrap();
    assert_eq!(cached.keys.len(), 1);
}
