#![cfg(not(miri))]
//! Wire-level token endpoint contracts checked against a loopback wiremock server.

use async_trait::async_trait;
use rullst_connect::client::{HttpClient, HttpRequest, HttpResponse};
use rullst_connect::provider::Provider;
use rullst_connect::providers::GithubProvider;
use secrecy::SecretString;
use std::sync::Arc;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Rewrites every provider host to the local wiremock server.
struct LoopbackClient {
    mock_server_url: String,
    inner: rullst_connect::client::ReqwestClient,
}

impl LoopbackClient {
    fn shared(mock_server_url: String) -> Arc<Self> {
        Arc::new(Self {
            mock_server_url,
            inner: rullst_connect::client::ReqwestClient::new(),
        })
    }
}

#[async_trait]
impl HttpClient for LoopbackClient {
    async fn execute(
        &self,
        mut request: HttpRequest,
    ) -> Result<HttpResponse, rullst_connect::error::ConnectError> {
        let parsed = url::Url::parse(&request.url).expect("provider URL");
        request.url = format!("{}{}", self.mock_server_url, parsed.path());
        if let Some(query) = parsed.query() {
            request.url.push('?');
            request.url.push_str(query);
        }
        self.inner.execute(request).await
    }
}

fn github(server: &MockServer) -> GithubProvider {
    GithubProvider::try_new(
        "github-client",
        SecretString::from("github-client-secret".to_string()),
        "https://app.example/callback",
    )
    .expect("live GitHub configuration")
    .with_http_client(LoopbackClient::shared(server.uri()))
}

#[tokio::test]
async fn github_refresh_requests_and_parses_a_json_token_response() {
    let server = MockServer::start().await;
    // GitHub answers form-encoded unless the client explicitly asks for JSON.
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .and(header("accept", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "ghu_rotated_access",
            "refresh_token": "ghr_rotated_refresh",
            "expires_in": 28_800,
            "token_type": "bearer"
        })))
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            "access_token=ghu_rotated_access&expires_in=28800&refresh_token=ghr_rotated_refresh",
            "application/x-www-form-urlencoded",
        ))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/user"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": 42,
            "login": "octocat"
        })))
        .mount(&server)
        .await;

    let refreshed = github(&server)
        .refresh_token("ghr_original_refresh")
        .await
        .expect("GitHub refresh parses the JSON token response");

    assert_eq!(refreshed.id, "42");
    assert_eq!(refreshed.expires_in, Some(28_800));
    assert!(
        refreshed
            .refresh_token
            .as_ref()
            .is_some_and(
                |token| secrecy::ExposeSecret::expose_secret(token) == "ghr_rotated_refresh"
            ),
        "the rotated refresh token must be returned"
    );
}

fn body_has_no_client_secret(request: &wiremock::Request) -> bool {
    !String::from_utf8_lossy(&request.body)
        .split('&')
        .any(|pair| pair.starts_with("client_secret="))
}

#[tokio::test]
async fn x_authenticates_confidential_token_requests_with_http_basic() {
    use base64::Engine as _;
    use rullst_connect::providers::XProvider;

    let server = MockServer::start().await;
    let basic = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode("x-client:x-client-secret")
    );
    // X rejects confidential clients that do not send an HTTP Basic header.
    Mock::given(method("POST"))
        .and(path("/2/oauth2/token"))
        .and(header("authorization", basic.as_str()))
        .and(body_has_no_client_secret)
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "x-access",
            "refresh_token": "x-refresh",
            "expires_in": 7200,
            "token_type": "bearer"
        })))
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/2/oauth2/token"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "error": "unauthorized_client",
            "error_description": "Missing valid authorization header"
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/2/users/me"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": { "id": "2244994945", "name": "X Dev" }
        })))
        .mount(&server)
        .await;

    let provider = XProvider::try_new(
        "x-client",
        SecretString::from("x-client-secret".to_string()),
        "https://app.example/callback",
    )
    .expect("live X configuration")
    .with_http_client(LoopbackClient::shared(server.uri()));

    let user = provider
        .get_user(rullst_connect::provider::ExchangeParams {
            auth_code: "x-code",
            code_verifier: Some("x-verifier"),
            ..Default::default()
        })
        .await
        .expect("code exchange uses client_secret_basic");
    assert_eq!(user.id, "2244994945");

    let refreshed = provider
        .refresh_token("x-refresh")
        .await
        .expect("refresh uses client_secret_basic");
    assert_eq!(refreshed.id, "2244994945");
}

#[tokio::test]
async fn a_profile_failure_after_rotation_keeps_the_issued_refresh_token() {
    use rullst_connect::{AutoRefreshingSession, ConnectError, ConnectUser};
    use secrecy::ExposeSecret as _;
    use wiremock::matchers::body_string_contains;

    let server = MockServer::start().await;
    for (sent, access, rotated) in [
        ("ghr-first", "ghu-second", "ghr-second"),
        ("ghr-second", "ghu-third", "ghr-third"),
    ] {
        Mock::given(method("POST"))
            .and(path("/login/oauth/access_token"))
            .and(body_string_contains(format!("refresh_token={sent}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": access,
                "refresh_token": rotated,
                "expires_in": 28_800
            })))
            .mount(&server)
            .await;
    }
    // The profile call right after the first rotation fails once. A 401 is
    // not retried by the optional `retry` transport, unlike a 5xx or 429.
    Mock::given(method("GET"))
        .and(path("/user"))
        .respond_with(ResponseTemplate::new(401))
        .up_to_n_times(1)
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/user"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": 42,
            "login": "octocat"
        })))
        .mount(&server)
        .await;

    let github = github(&server);
    let signed_in = ConnectUser {
        id: "42".to_string(),
        name: "octocat".to_string(),
        email: None,
        email_verified: None,
        avatar_url: None,
        raw_data: serde_json::json!({}),
        access_token: SecretString::from("ghu-first".to_string()),
        refresh_token: Some(SecretString::from("ghr-first".to_string())),
        expires_in: Some(10),
    };
    let session = AutoRefreshingSession::from_user_at(&github, &signed_in, 1_000)
        .expect("session")
        .with_refresh_leeway(0)
        .expect("leeway");

    let error = session
        .access_token_at(1_010)
        .await
        .expect_err("the profile lookup failed");
    assert!(matches!(error, ConnectError::ProviderApiError { .. }));
    let state = session.state_snapshot().await;
    assert!(
        state.refresh_token().expose_secret() == "ghr-second",
        "the rotated refresh token must survive the profile failure"
    );

    let lease = session
        .access_token_at(1_010)
        .await
        .expect("the next call refreshes with the rotation");
    assert!(lease.access_token().expose_secret() == "ghu-third");
    assert_eq!(lease.generation(), 2);
}

#[tokio::test]
async fn direct_refresh_callers_receive_the_issued_tokens() {
    use rullst_connect::ConnectError;
    use secrecy::ExposeSecret as _;

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "ghu-issued",
            "refresh_token": "ghr-issued",
            "expires_in": 28_800
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/user"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&server)
        .await;

    let error = github(&server)
        .refresh_token("ghr-original")
        .await
        .expect_err("profile lookup rejected");
    let ConnectError::RefreshIncomplete { tokens, source } = error else {
        panic!("expected RefreshIncomplete");
    };
    assert!(tokens.access_token().expose_secret() == "ghu-issued");
    assert!(
        tokens
            .refresh_token()
            .is_some_and(|token| token.expose_secret() == "ghr-issued")
    );
    assert_eq!(tokens.expires_in(), Some(28_800));
    assert!(matches!(*source, ConnectError::ProviderApiError { .. }));
    assert!(!format!("{tokens:?}").contains("ghr-issued"));
}
