#![cfg(not(miri))]
use async_trait::async_trait;
use rullst_connect::client::{HttpClient, HttpRequest, HttpResponse};
use rullst_connect::error::ConnectError;
use rullst_connect::provider::Provider;
use rullst_connect::providers::GithubProvider;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Intercepts all requests and rewrites the host to point to the wiremock local server
struct WiremockInterceptClient {
    mock_server_url: String,
    inner: rullst_connect::client::ReqwestClient,
}

impl WiremockInterceptClient {
    fn new(mock_server_url: String) -> Self {
        Self {
            mock_server_url,
            inner: rullst_connect::client::ReqwestClient::new(),
        }
    }
}

#[async_trait]
impl HttpClient for WiremockInterceptClient {
    async fn execute(
        &self,
        mut req: HttpRequest,
    ) -> Result<HttpResponse, rullst_connect::error::ConnectError> {
        let parsed = url::Url::parse(&req.url).unwrap();
        // Redirect the request to our mock server instead of github.com or api.github.com
        req.url = format!("{}{}", self.mock_server_url, parsed.path());
        self.inner.execute(req).await
    }
}

#[tokio::test]
async fn test_github_get_user_success() {
    let mock_server = MockServer::start().await;

    // 1. Mock the token exchange endpoint
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "mock_access_token_123",
            "token_type": "bearer",
            "scope": "repo,gist",
            "refresh_token": "mock_refresh_token_abc",
            "expires_in": 3600
        })))
        .mount(&mock_server)
        .await;

    // 2. Mock the user profile endpoint
    Mock::given(method("GET"))
        .and(path("/user"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": 123456,
            "login": "octocat",
            "name": "The Octocat",
            "email": "octocat@github.com",
            "avatar_url": "https://github.com/images/error/octocat_happy.gif",
        })))
        .mount(&mock_server)
        .await;

    // 3. Create Provider with our intercepted Mock Client
    let intercept_client = std::sync::Arc::new(WiremockInterceptClient::new(mock_server.uri()));
    let provider = GithubProvider::new(
        "test_client_id".to_string(),
        secrecy::SecretString::from("test_client_secret".to_string()),
        "http://localhost/callback".to_string(),
    )
    .with_http_client(intercept_client);

    // 4. Perform the full get_user flow!
    let params = rullst_connect::provider::ExchangeParams {
        auth_code: "fake_auth_code_999",
        ..Default::default()
    };
    let user = provider.get_user(params).await.unwrap();

    assert_eq!(user.id, "123456");
    assert_eq!(user.name, "The Octocat");
    assert_eq!(user.email.as_deref(), Some("octocat@github.com"));
    assert_eq!(
        user.avatar_url.as_deref(),
        Some("https://github.com/images/error/octocat_happy.gif")
    );
    assert_eq!(
        secrecy::ExposeSecret::expose_secret(&user.access_token),
        "mock_access_token_123"
    );
    assert_eq!(
        user.refresh_token
            .as_ref()
            .map(secrecy::ExposeSecret::expose_secret),
        Some("mock_refresh_token_abc")
    );
    assert_eq!(user.expires_in, Some(3600));
}

#[tokio::test]
async fn test_github_token_error() {
    let mock_server = MockServer::start().await;

    // Mock an error response from the provider during token exchange
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error": "invalid_grant",
            "error_description": "The code passed is incorrect or expired."
        })))
        .mount(&mock_server)
        .await;

    let intercept_client = std::sync::Arc::new(WiremockInterceptClient::new(mock_server.uri()));
    let provider = GithubProvider::new(
        "test_client_id".to_string(),
        secrecy::SecretString::from("test_client_secret".to_string()),
        "http://localhost/callback".to_string(),
    )
    .with_http_client(intercept_client);

    let params = rullst_connect::provider::ExchangeParams {
        auth_code: "bad_code",
        ..Default::default()
    };
    let err = provider.get_user(params).await.unwrap_err();

    assert!(matches!(
        err,
        ConnectError::ProviderApiError { ref code, ref message }
            if code == "invalid_grant"
                && message == "The code passed is incorrect or expired."
    ));
}

#[tokio::test]
async fn test_github_request_device_code_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/login/device/code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_code": "mock_device_code_123",
            "user_code": "WDWD-XXXX",
            "verification_uri": "https://github.com/login/device",
            "expires_in": 900,
            "interval": 5
        })))
        .mount(&mock_server)
        .await;

    let intercept_client = std::sync::Arc::new(WiremockInterceptClient::new(mock_server.uri()));
    let provider = GithubProvider::new(
        "test_client_id".to_string(),
        secrecy::SecretString::from("test_client_secret".to_string()),
        "http://localhost/callback".to_string(),
    )
    .with_http_client(intercept_client);

    let res = provider.request_device_code().await.unwrap();
    assert_eq!(res.device_code, "mock_device_code_123");
    assert_eq!(res.user_code, "WDWD-XXXX");
    assert_eq!(res.verification_uri, "https://github.com/login/device");
    assert_eq!(res.expires_in, 900);
    assert_eq!(res.interval, Some(5));
}

#[tokio::test]
async fn test_github_refresh_token_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "error": "invalid_grant",
            "error_description": "The refresh token is invalid."
        })))
        .mount(&mock_server)
        .await;

    let intercept_client = std::sync::Arc::new(WiremockInterceptClient::new(mock_server.uri()));
    let provider = GithubProvider::new(
        "test_client_id".to_string(),
        secrecy::SecretString::from("test_client_secret".to_string()),
        "http://localhost/callback".to_string(),
    )
    .with_http_client(intercept_client);

    let err = provider
        .refresh_token("bad_refresh_token")
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        ConnectError::Token(ref message)
            if message.contains("Provider returned error: invalid_grant - The refresh token is invalid.")
    ));
}

#[tokio::test]
async fn test_github_get_user_missing_access_token() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "token_type": "bearer"
        })))
        .mount(&mock_server)
        .await;

    let intercept_client = std::sync::Arc::new(WiremockInterceptClient::new(mock_server.uri()));
    let provider = GithubProvider::new(
        "test_client_id".to_string(),
        secrecy::SecretString::from("test_client_secret".to_string()),
        "http://localhost/callback".to_string(),
    )
    .with_http_client(intercept_client);

    let params = rullst_connect::provider::ExchangeParams {
        auth_code: "some_code",
        ..Default::default()
    };
    let err = provider.get_user(params).await.unwrap_err();
    assert!(matches!(
        err,
        ConnectError::Token(ref message)
            if message.contains("Failed to get access_token")
    ));
}

#[tokio::test]
async fn test_github_get_user_missing_id() {
    let mock_server = MockServer::start().await;

    // 1. Mock token exchange (success)
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "mock_access_token_123"
        })))
        .mount(&mock_server)
        .await;

    // 2. Mock user profile but without ID
    Mock::given(method("GET"))
        .and(path("/user"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "login": "octocat",
            "name": "The Octocat"
        })))
        .mount(&mock_server)
        .await;

    let intercept_client = std::sync::Arc::new(WiremockInterceptClient::new(mock_server.uri()));
    let provider = GithubProvider::new(
        "test_client_id".to_string(),
        secrecy::SecretString::from("test_client_secret".to_string()),
        "http://localhost/callback".to_string(),
    )
    .with_http_client(intercept_client);

    let params = rullst_connect::provider::ExchangeParams {
        auth_code: "some_code",
        ..Default::default()
    };
    let err = provider.get_user(params).await.unwrap_err();
    assert!(matches!(
        err,
        ConnectError::Provider(ref message)
            if message.contains("Missing id")
    ));
}
