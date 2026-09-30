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
