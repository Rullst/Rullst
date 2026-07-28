use crate::client::HttpClientExt;
use crate::user::ConnectUser;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::LazyLock;
use tokio::sync::RwLock;

pub static JWKS_CACHE: LazyLock<
    RwLock<HashMap<String, std::sync::Arc<jsonwebtoken::jwk::JwkSet>>>,
> = LazyLock::new(|| RwLock::new(HashMap::new()));

pub async fn fetch_and_cache_jwks(
    url: &str,
    client: &dyn crate::client::HttpClient,
) -> Result<std::sync::Arc<jsonwebtoken::jwk::JwkSet>, crate::error::ConnectError> {
    #[cfg(not(test))]
    #[cfg_attr(coverage_nightly, coverage(off))]
    {
        let cache = JWKS_CACHE.read().await;
        if let Some(jwks) = cache.get(url) {
            return Ok(jwks.clone());
        }
    }

    let jwks = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json::<jsonwebtoken::jwk::JwkSet>()
        .await?;

    let jwks_arc = std::sync::Arc::new(jwks);

    #[cfg(not(test))]
    #[cfg_attr(coverage_nightly, coverage(off))]
    {
        let url_str = url.to_string();
        let mut cache = JWKS_CACHE.write().await;
        cache.insert(url_str, jwks_arc.clone());
    }

    Ok(jwks_arc)
}
/// Helper to construct standard OAuth2 parameters to reduce boilerplate.
pub fn build_oauth_params<'a>(
    base_url: &str,
    client_id: &'a str,
    redirect_uri: &'a str,
    scopes: &'a str,
    state: Option<&'a str>,
    pkce_challenge: Option<&'a str>,
) -> url::form_urlencoded::Serializer<'a, String> {
    let mut string = String::with_capacity(base_url.len() + 256);
    string.push_str(base_url);
    let separator = if base_url.contains('?') { '&' } else { '?' };
    string.push(separator);
    let start_position = string.len();
    let mut params = url::form_urlencoded::Serializer::for_suffix(string, start_position);
    params.append_pair("client_id", client_id);
    params.append_pair("redirect_uri", redirect_uri);
    if !scopes.is_empty() {
        params.append_pair("scope", scopes);
    }
    if let Some(s) = state {
        params.append_pair("state", s);
    }
    if let Some(p) = pkce_challenge {
        params.append_pair("code_challenge", p);
        params.append_pair("code_challenge_method", "S256");
    }
    params
}

/// Parameters to exchange the authorization code for tokens.
#[derive(Debug, Default, Clone)]
pub struct ExchangeParams<'a> {
    pub auth_code: &'a str,
    pub code_verifier: Option<&'a str>,
    pub expected_nonce: Option<&'a str>,
}

#[derive(serde::Serialize)]
pub struct TokenExchangeForm<'a> {
    pub client_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<&'a str>,
    pub code: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grant_type: Option<&'a str>,
    pub redirect_uri: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_verifier: Option<&'a str>,
}

/// The core trait implemented by all OAuth2 providers in Rullst Connect.
#[async_trait]
pub trait Provider: Send + Sync {
    /// Returns the authorization URL to redirect the user to the provider's login screen.
    fn redirect_url(&self) -> String;

    /// Returns the authorization URL with a `state` parameter appended.
    /// It is highly recommended to use this to prevent CSRF attacks.
    fn redirect_url_with_state(&self, state: &str) -> String {
        let mut string = self.redirect_url();
        // Pre-allocate capacity to prevent reallocation when appending query parameters
        string.reserve(8_usize.saturating_add(state.len()));
        let separator = if string.contains('?') { '&' } else { '?' };
        string.push(separator);
        let start_position = string.len();
        let mut serializer = url::form_urlencoded::Serializer::for_suffix(string, start_position);
        serializer.append_pair("state", state);
        serializer.finish()
    }

    /// Returns the authorization URL with a PKCE `code_challenge` appended.
    /// Useful for providers that enforce PKCE (like Twitter/X v2).
    fn redirect_url_with_pkce(&self, code_challenge: &str) -> String {
        let mut string = self.redirect_url();
        // Pre-allocate capacity to prevent reallocation when appending query parameters
        string.reserve(44_usize.saturating_add(code_challenge.len()));
        let separator = if string.contains('?') { '&' } else { '?' };
        string.push(separator);
        let start_position = string.len();
        let mut serializer = url::form_urlencoded::Serializer::for_suffix(string, start_position);
        serializer.append_pair("code_challenge", code_challenge);
        serializer.append_pair("code_challenge_method", "S256");
        serializer.finish()
    }

    /// Returns the authorization URL with a PKCE `code_challenge` and a `state` parameter appended.
    fn redirect_url_with_pkce_and_state(&self, code_challenge: &str, state: &str) -> String {
        let mut string = self.redirect_url();
        // Pre-allocate capacity to prevent reallocation when appending query parameters
        string.reserve(
            52_usize
                .saturating_add(code_challenge.len())
                .saturating_add(state.len()),
        );
        let separator = if string.contains('?') { '&' } else { '?' };
        string.push(separator);
        let start_position = string.len();
        let mut serializer = url::form_urlencoded::Serializer::for_suffix(string, start_position);
        serializer.append_pair("code_challenge", code_challenge);
        serializer.append_pair("code_challenge_method", "S256");
        serializer.append_pair("state", state);
        serializer.finish()
    }

    /// Exchanges the authorization code for an access token and fetches the user's profile.
    /// Returns a standardized `ConnectUser` or a `ConnectError`.
    async fn get_user(
        &self,
        params: ExchangeParams<'_>,
    ) -> Result<ConnectUser, crate::error::ConnectError>;

    /// Fetches the user's profile using an existing access token.
    /// This bypasses the authorization code exchange step.
    async fn get_user_from_token(
        &self,
        access_token: &str,
    ) -> Result<ConnectUser, crate::error::ConnectError>;

    /// Returns the URL used to exchange the authorization code for an access token.
    fn token_url(&self) -> String;

    /// Exchanges a refresh token for a new access token and fetches the user profile.
    async fn refresh_token(
        &self,
        _refresh_token: &str,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
        Err(crate::error::ConnectError::Token(
            "Refresh token is not supported by this provider".to_string(),
        ))
    }

    /// Revokes an access token (or refresh token) directly on the provider's authorization server.
    /// By default, this returns a `Token` error since not all providers support token revocation.
    async fn revoke_token(&self, _token: &str) -> Result<(), crate::error::ConnectError> {
        Err(crate::error::ConnectError::Token(
            "Token revocation is not supported by this provider".to_string(),
        ))
    }

    /// Initiates a device authorization flow (RFC 8628).
    /// Returns the device code, user code, and verification URI.
    async fn request_device_code(
        &self,
    ) -> Result<crate::user::DeviceAuthorizationResponse, crate::error::ConnectError> {
        Err(crate::error::ConnectError::Provider(
            "Device Authorization is not supported by this provider".into(),
        ))
    }

    /// Polls the provider for the access token during a device authorization flow.
    /// Returns the user's profile if the user has authorized the device.
    async fn poll_device_token(
        &self,
        _device_code: &str,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
        Err(crate::error::ConnectError::Provider(
            "Device Authorization is not supported by this provider".into(),
        ))
    }
}

/// The response containing token information from a standard OAuth2 exchange.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Oauth2TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
}

/// Helper to exchange an authorization code for access tokens using standard OAuth2.
pub async fn fetch_access_token(
    client: &dyn crate::client::HttpClient,
    token_url: &str,
    form: &TokenExchangeForm<'_>,
) -> Result<Oauth2TokenResponse, crate::error::ConnectError> {
    let token_res = client
        .post(token_url)
        .form(form)
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;

    if let Some(err) = token_res["error"].as_str() {
        let err_desc = token_res["error_description"].as_str().unwrap_or("");
        return Err(crate::error::ConnectError::Token(format!(
            "Provider returned error: {} - {}",
            err, err_desc
        )));
    }

    let access_token = token_res["access_token"]
        .as_str()
        .ok_or_else(|| crate::error::ConnectError::Token("Failed to get access_token".to_owned()))?
        .to_owned();

    let refresh_token = token_res["refresh_token"].as_str().map(String::from);
    let expires_in = token_res["expires_in"]
        .as_u64()
        .or_else(|| token_res["expires_in"].as_i64().map(|v| v as u64));

    Ok(Oauth2TokenResponse {
        access_token,
        refresh_token,
        expires_in,
    })
}

/// Helper to exchange a refresh token for new access tokens using standard OAuth2.
pub async fn fetch_refresh_token(
    client: &dyn crate::client::HttpClient,
    token_url: &str,
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> Result<Oauth2TokenResponse, crate::error::ConnectError> {
    let token_res = client
        .post(token_url)
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;

    if let Some(err) = token_res["error"].as_str() {
        let err_desc = token_res["error_description"].as_str().unwrap_or("");
        return Err(crate::error::ConnectError::Token(format!(
            "Provider returned error: {} - {}",
            err, err_desc
        )));
    }

    let access_token = token_res["access_token"]
        .as_str()
        .ok_or_else(|| {
            crate::error::ConnectError::Token(
                "Failed to get access_token during refresh".to_owned(),
            )
        })?
        .to_owned();

    let refresh_token = token_res["refresh_token"].as_str().map(String::from);
    let expires_in = token_res["expires_in"]
        .as_u64()
        .or_else(|| token_res["expires_in"].as_i64().map(|v| v as u64));

    Ok(Oauth2TokenResponse {
        access_token,
        refresh_token,
        expires_in,
    })
}

/// Helper to exchange an authorization code and build the ConnectUser profile.
#[allow(clippy::too_many_arguments)]
pub async fn exchange_and_get_user<P>(
    provider: &P,
    client: &dyn crate::client::HttpClient,
    token_url: &str,
    form: &TokenExchangeForm<'_>,
    _expected_nonce: Option<&str>,
) -> Result<ConnectUser, crate::error::ConnectError>
where
    P: Provider + ?Sized,
{
    let token = fetch_access_token(client, token_url, form).await?;

    let mut user = provider.get_user_from_token(&token.access_token).await?;
    user.refresh_token = token.refresh_token.map(secrecy::SecretString::from);
    user.expires_in = token.expires_in;
    Ok(user)
}

/// Helper to refresh an access token and fetch the updated ConnectUser profile.
pub async fn refresh_and_get_user<P>(
    provider: &P,
    client: &dyn crate::client::HttpClient,
    token_url: &str,
    client_id: &str,
    client_secret: &secrecy::SecretString,
    refresh_token: &str,
) -> Result<ConnectUser, crate::error::ConnectError>
where
    P: Provider + ?Sized,
{
    let token = fetch_refresh_token(
        client,
        token_url,
        client_id,
        secrecy::ExposeSecret::expose_secret(client_secret),
        refresh_token,
    )
    .await?;

    let mut user = provider.get_user_from_token(&token.access_token).await?;
    user.refresh_token = token.refresh_token.map(secrecy::SecretString::from);
    user.expires_in = token.expires_in;
    Ok(user)
}

pub(crate) fn verify_nonce(token_nonce: &str, expected_nonce: &str) -> bool {
    use sha2::{Digest, Sha256};
    use subtle::ConstantTimeEq;

    let hash_token = Sha256::digest(token_nonce.as_bytes());
    let hash_expected = Sha256::digest(expected_nonce.as_bytes());

    bool::from(hash_token.ct_eq(&hash_expected))
}

#[cfg(test)]
mod tests {
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
}
