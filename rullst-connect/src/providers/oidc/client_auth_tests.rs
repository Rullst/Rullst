//! OIDC token-endpoint contracts: client authentication selected from
//! discovery and tokens carried out of an incomplete refresh.

use super::discovery::OidcProvider;
use crate::client::{HttpClient, HttpRequest, HttpResponse};
use crate::error::ConnectError;
use crate::provider::Provider;
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

/// Records token requests and answers like a discovery-backed provider.
struct RecordingClient {
    auth_methods: Option<Value>,
    token_requests: Mutex<Vec<HttpRequest>>,
    userinfo_status: u16,
}

#[async_trait]
impl HttpClient for RecordingClient {
    async fn execute(&self, request: HttpRequest) -> Result<HttpResponse, ConnectError> {
        let body = if request.url.ends_with("/.well-known/openid-configuration") {
            let mut metadata = json!({
                "issuer": "https://issuer.example",
                "authorization_endpoint": "https://issuer.example/authorize",
                "token_endpoint": "https://issuer.example/token",
                "userinfo_endpoint": "https://issuer.example/userinfo",
                "jwks_uri": "https://issuer.example/jwks"
            });
            if let Some(methods) = &self.auth_methods {
                metadata["token_endpoint_auth_methods_supported"] = methods.clone();
            }
            metadata
        } else if request.url.ends_with("/token") {
            self.token_requests.lock().unwrap().push(request);
            json!({
                "access_token": "issued-access",
                "refresh_token": "issued-refresh",
                "expires_in": 3600
            })
        } else if request.url.ends_with("/userinfo") {
            if self.userinfo_status != 200 {
                return Ok(HttpResponse {
                    status: self.userinfo_status,
                    body: json!({ "error": "temporarily_unavailable" }),
                });
            }
            json!({ "sub": "user-1", "name": "Ada" })
        } else {
            return Err(ConnectError::Provider("unexpected URL".to_string()));
        };
        Ok(HttpResponse { status: 200, body })
    }
}

async fn provider(auth_methods: Option<Value>) -> (OidcProvider, Arc<RecordingClient>) {
    provider_with_userinfo(auth_methods, 200).await
}

async fn provider_with_userinfo(
    auth_methods: Option<Value>,
    userinfo_status: u16,
) -> (OidcProvider, Arc<RecordingClient>) {
    let client = Arc::new(RecordingClient {
        auth_methods,
        token_requests: Mutex::new(Vec::new()),
        userinfo_status,
    });
    let provider = OidcProvider::discover_with_client(
        "https://issuer.example",
        "oidc-client",
        "oidc-client-secret",
        "https://app.example/callback",
        client.clone(),
    )
    .await
    .unwrap();
    (provider, client)
}

async fn exchange_and_refresh(provider: &OidcProvider) {
    provider
        .get_user(crate::provider::ExchangeParams {
            auth_code: "code",
            ..Default::default()
        })
        .await
        .unwrap();
    provider.refresh_token("prior-refresh").await.unwrap();
}

fn body_carries_secret(request: &HttpRequest) -> bool {
    request
        .form
        .as_deref()
        .unwrap_or_default()
        .split('&')
        .any(|pair| pair.starts_with("client_secret="))
}

#[tokio::test]
async fn a_basic_only_provider_receives_http_basic_credentials() {
    let (provider, client) = provider(Some(json!(["client_secret_basic"]))).await;
    exchange_and_refresh(&provider).await;

    let requests = client.token_requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    for request in requests.iter() {
        let (username, password) = request.basic_auth.as_ref().expect("HTTP Basic credentials");
        assert_eq!(username, "oidc-client");
        assert!(password.as_deref() == Some("oidc-client-secret"));
        assert!(
            !body_carries_secret(request),
            "the secret must not be in the body"
        );
    }
}

#[tokio::test]
async fn post_stays_selected_when_advertised_or_unspecified() {
    for methods in [
        None,
        Some(json!(["client_secret_post"])),
        Some(json!(["client_secret_basic", "client_secret_post"])),
    ] {
        let (provider, client) = provider(methods).await;
        exchange_and_refresh(&provider).await;

        let requests = client.token_requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        for request in requests.iter() {
            assert!(request.basic_auth.is_none());
            assert!(body_carries_secret(request));
        }
    }
}

#[tokio::test]
async fn a_refresh_whose_userinfo_fails_carries_the_issued_tokens() {
    use secrecy::ExposeSecret as _;

    let (provider, _client) = provider_with_userinfo(None, 403).await;
    let error = provider.refresh_token("prior-refresh").await.unwrap_err();
    let ConnectError::RefreshIncomplete { tokens, source } = error else {
        panic!("expected RefreshIncomplete");
    };
    assert!(tokens.access_token().expose_secret() == "issued-access");
    assert!(
        tokens
            .refresh_token()
            .is_some_and(|token| token.expose_secret() == "issued-refresh")
    );
    assert_eq!(tokens.expires_in(), Some(3600));
    assert!(matches!(*source, ConnectError::ProviderApiError { .. }));

    // A failed code exchange is a login failure, not an incomplete refresh.
    let login = provider
        .get_user(crate::provider::ExchangeParams {
            auth_code: "code",
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert!(matches!(login, ConnectError::ProviderApiError { .. }));
}
