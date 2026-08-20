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

#[test]
fn test_pkce_generation() {
    let (verifier, challenge) = rullst_connect::pkce::generate_pkce();
    assert_eq!(verifier.len(), 64);
    assert!(!challenge.is_empty());
    assert!(!challenge.contains('='));
}

#[test]
fn test_providers_initialization() {
    use rullst_connect::provider::Provider;
    use rullst_connect::providers::*;

    let gh = GithubProvider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("secret".to_string()),
        "http://localhost/callback".to_string(),
    );
    assert!(gh.redirect_url().contains("github.com"));
    assert!(gh.redirect_url_with_state("xyz").contains("state=xyz"));

    let google = GoogleProvider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("secret".to_string()),
        "http://localhost/callback".to_string(),
    );
    assert!(google.redirect_url().contains("google.com"));

    let discord = DiscordProvider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("secret".to_string()),
        "http://localhost/callback".to_string(),
    );
    assert!(discord.redirect_url().contains("discord.com"));

    let ms = MicrosoftProvider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("secret".to_string()),
        "http://localhost/callback".to_string(),
    );
    assert!(ms.redirect_url().contains("microsoftonline.com"));

    let fb = FacebookProvider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("secret".to_string()),
        "http://localhost/callback".to_string(),
    );
    assert!(fb.redirect_url().contains("facebook.com"));

    let li = LinkedinProvider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("secret".to_string()),
        "http://localhost/callback".to_string(),
    );
    assert!(li.redirect_url().contains("linkedin.com"));

    let x = XProvider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("secret".to_string()),
        "http://localhost/callback".to_string(),
    );
    assert!(x.redirect_url().contains("twitter.com") || x.redirect_url().contains("x.com"));

    let auth0 = Auth0Provider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("secret".to_string()),
        "http://localhost/callback".to_string(),
        "dev-domain.auth0.com".to_string(),
    );
    assert!(auth0.redirect_url().contains("dev-domain.auth0.com"));

    let cognito = CognitoProvider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("secret".to_string()),
        "http://localhost/callback".to_string(),
        "https://auth.us-east-1.amazoncognito.com".to_string(),
    );
    assert!(cognito.redirect_url().contains("amazoncognito.com"));
}

#[tokio::test]
async fn test_google_get_user_wiremock() {
    use rullst_connect::providers::GoogleProvider;
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "google_access_token_123",
            "token_type": "Bearer",
            "expires_in": 3600,
            "refresh_token": "google_refresh_token_456"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/oauth2/v3/userinfo"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "sub": "google-user-999",
            "name": "Jane Google",
            "email": "jane@gmail.com",
            "email_verified": true,
            "picture": "https://lh3.googleusercontent.com/a/avatar.jpg"
        })))
        .mount(&mock_server)
        .await;

    let intercept_client = std::sync::Arc::new(WiremockInterceptClient::new(mock_server.uri()));
    let provider = GoogleProvider::new(
        "test_client_id".to_string(),
        secrecy::SecretString::from("test_client_secret".to_string()),
        "http://localhost/callback".to_string(),
    )
    .with_http_client(intercept_client);

    let params = rullst_connect::provider::ExchangeParams {
        auth_code: "google_code",
        ..Default::default()
    };
    let user = provider.get_user(params).await.unwrap();
    assert_eq!(user.id, "google-user-999");
    assert_eq!(user.name, "Jane Google");
    assert_eq!(user.email.as_deref(), Some("jane@gmail.com"));
    assert_eq!(user.email_verified, Some(true));
}

#[tokio::test]
async fn test_discord_get_user_wiremock() {
    use rullst_connect::providers::DiscordProvider;
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/oauth2/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "discord_access_token_123",
            "token_type": "Bearer",
            "expires_in": 604800
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/users/@me"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "80351110224678912",
            "username": "Nelly",
            "discriminator": "1337",
            "email": "nelly@discord.com",
            "verified": true,
            "avatar": "8342729096ea3686c6015e5b5ff12d4e"
        })))
        .mount(&mock_server)
        .await;

    let intercept_client = std::sync::Arc::new(WiremockInterceptClient::new(mock_server.uri()));
    let provider = DiscordProvider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("secret".to_string()),
        "http://localhost/callback".to_string(),
    )
    .with_http_client(intercept_client);

    let params = rullst_connect::provider::ExchangeParams {
        auth_code: "discord_code",
        ..Default::default()
    };
    let user = provider.get_user(params).await.unwrap();
    assert_eq!(user.id, "80351110224678912");
    assert_eq!(user.email.as_deref(), Some("nelly@discord.com"));
}

#[tokio::test]
async fn test_facebook_get_user_wiremock() {
    use rullst_connect::providers::FacebookProvider;
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v19.0/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "fb_access_token_123",
            "token_type": "bearer",
            "expires_in": 5184000
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v19.0/me"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "10000123456789",
            "name": "Mark Developer",
            "email": "mark@example.com"
        })))
        .mount(&mock_server)
        .await;

    let intercept_client = std::sync::Arc::new(WiremockInterceptClient::new(mock_server.uri()));
    let provider = FacebookProvider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("secret".to_string()),
        "http://localhost/callback".to_string(),
    )
    .with_http_client(intercept_client);

    let params = rullst_connect::provider::ExchangeParams {
        auth_code: "fb_code",
        ..Default::default()
    };
    let user = provider.get_user(params).await.unwrap();
    assert_eq!(user.id, "10000123456789");
    assert_eq!(user.name, "Mark Developer");
}

#[tokio::test]
async fn test_microsoft_get_user_wiremock() {
    use rullst_connect::providers::MicrosoftProvider;
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/common/oauth2/v2.0/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "ms_access_token_123",
            "token_type": "Bearer",
            "expires_in": 3600
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1.0/me"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "ms-user-guid-1234",
            "displayName": "Satya Azure",
            "mail": "satya@microsoft.com"
        })))
        .mount(&mock_server)
        .await;

    let intercept_client = std::sync::Arc::new(WiremockInterceptClient::new(mock_server.uri()));
    let provider = MicrosoftProvider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("secret".to_string()),
        "http://localhost/callback".to_string(),
    )
    .with_http_client(intercept_client);

    let params = rullst_connect::provider::ExchangeParams {
        auth_code: "ms_code",
        ..Default::default()
    };
    let user = provider.get_user(params).await.unwrap();
    assert_eq!(user.id, "ms-user-guid-1234");
    assert_eq!(user.name, "Satya Azure");
}

#[tokio::test]
async fn test_linkedin_get_user_wiremock() {
    use rullst_connect::providers::LinkedinProvider;
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/oauth/v2/accessToken"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "li_access_token_123",
            "expires_in": 5184000
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v2/userinfo"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "sub": "li-sub-123",
            "name": "Reid Professional",
            "email": "reid@linkedin.com",
            "picture": "https://media.licdn.com/dms/image/profile.jpg"
        })))
        .mount(&mock_server)
        .await;

    let intercept_client = std::sync::Arc::new(WiremockInterceptClient::new(mock_server.uri()));
    let provider = LinkedinProvider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("secret".to_string()),
        "http://localhost/callback".to_string(),
    )
    .with_http_client(intercept_client);

    let params = rullst_connect::provider::ExchangeParams {
        auth_code: "li_code",
        ..Default::default()
    };
    let user = provider.get_user(params).await.unwrap();
    assert_eq!(user.id, "li-sub-123");
    assert_eq!(user.name, "Reid Professional");
}

#[tokio::test]
async fn test_x_get_user_wiremock() {
    use rullst_connect::providers::XProvider;
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/2/oauth2/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "x_access_token_123",
            "token_type": "bearer",
            "expires_in": 7200
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/2/users/me"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "id": "2244994945",
                "name": "Twitter Dev",
                "username": "TwitterDev"
            }
        })))
        .mount(&mock_server)
        .await;

    let intercept_client = std::sync::Arc::new(WiremockInterceptClient::new(mock_server.uri()));
    let provider = XProvider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("secret".to_string()),
        "http://localhost/callback".to_string(),
    )
    .with_http_client(intercept_client);

    let params = rullst_connect::provider::ExchangeParams {
        auth_code: "x_code",
        code_verifier: Some("verifier_123"),
        ..Default::default()
    };
    let user = provider.get_user(params).await.unwrap();
    assert_eq!(user.id, "2244994945");
    assert_eq!(user.name, "Twitter Dev");
}

#[tokio::test]
async fn test_auth0_get_user_wiremock() {
    use rullst_connect::providers::Auth0Provider;
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "auth0_access_token_123",
            "token_type": "Bearer",
            "expires_in": 86400
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/userinfo"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "sub": "auth0|607f1f77bcf86cd799439011",
            "name": "Auth0 User",
            "email": "user@auth0.com",
            "email_verified": true
        })))
        .mount(&mock_server)
        .await;

    let intercept_client = std::sync::Arc::new(WiremockInterceptClient::new(mock_server.uri()));
    let provider = Auth0Provider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("secret".to_string()),
        "http://localhost/callback".to_string(),
        "test-tenant.auth0.com".to_string(),
    )
    .with_http_client(intercept_client);

    let params = rullst_connect::provider::ExchangeParams {
        auth_code: "auth0_code",
        ..Default::default()
    };
    let user = provider.get_user(params).await.unwrap();
    assert_eq!(user.id, "auth0|607f1f77bcf86cd799439011");
    assert_eq!(user.email.as_deref(), Some("user@auth0.com"));
}

#[cfg(feature = "axum")]
#[tokio::test]
async fn test_mock_idp_router_execution() {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use rullst_connect::mock_idp::mock_router;
    use tower::ServiceExt;

    let app = mock_router();

    // 1. Test GET /auth
    let req = Request::builder()
        .uri("/auth?client_id=cid&redirect_uri=http://localhost/cb&response_type=code&state=xyz")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_redirection());

    // 2. Test GET /.well-known/openid-configuration
    let req = Request::builder()
        .uri("/.well-known/openid-configuration")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 3. Test POST /token
    let form = "client_id=cid&client_secret=sec&code=mock_code&grant_type=authorization_code&redirect_uri=http://localhost/cb";
    let req = Request::builder()
        .method("POST")
        .uri("/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(Body::from(form))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 4. Test GET /userinfo
    let req = Request::builder()
        .uri("/userinfo")
        .header("Authorization", "Bearer mock_token")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[test]
fn test_connect_user_serialization() {
    use rullst_connect::user::ConnectUser;
    let user = ConnectUser {
        id: "usr_123".to_string(),
        name: "Test User".to_string(),
        email: Some("test@example.com".to_string()),
        email_verified: Some(true),
        avatar_url: Some("https://example.com/avatar.png".to_string()),
        raw_data: serde_json::json!({"provider_raw": true}),
        access_token: secrecy::SecretString::from("tok_secret".to_string()),
        refresh_token: Some(secrecy::SecretString::from("ref_secret".to_string())),
        expires_in: Some(3600),
    };

    let serialized = serde_json::to_string(&user).unwrap();
    assert!(serialized.contains("usr_123"));
    assert!(serialized.contains("tok_secret"));

    let deserialized: ConnectUser = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized.id, "usr_123");
    assert_eq!(deserialized.name, "Test User");
}

#[test]
fn test_connect_error_variants() {
    let err_req = ConnectError::Reqwest("Connection reset".to_string());
    assert!(format!("{}", err_req).contains("HTTP request failed: Connection reset"));

    let err_tok = ConnectError::Token("Expired".to_string());
    assert!(format!("{}", err_tok).contains("Missing token or unexpected response: Expired"));

    let err_state = ConnectError::InvalidState("Mismatch".to_string());
    assert!(format!("{}", err_state).contains("Invalid CSRF state: Mismatch"));
}
