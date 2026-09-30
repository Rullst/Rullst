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
