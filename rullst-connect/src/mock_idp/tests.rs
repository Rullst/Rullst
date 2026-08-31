use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use rullst_connect_test_support::NoRedirectClient;
use serde_json::Value;
use tower::ServiceExt;

use super::*;

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("bounded mock IdP response body");
    serde_json::from_slice(&bytes).expect("valid mock IdP JSON")
}

#[test]
fn config_is_local_only_and_bounded() {
    assert!(
        MockIdpConfig::try_new(
            "https://identity.example.com",
            "client",
            "secret",
            "http://127.0.0.1:3000/callback"
        )
        .is_err()
    );
    assert!(
        MockIdpConfig::try_new(
            "http://localhost.evil:8080",
            "client",
            "secret",
            "http://127.0.0.1:3000/callback"
        )
        .is_err()
    );
    assert!(
        MockIdpConfig::try_new(
            "http://127.0.0.1:8080",
            "client",
            "secret",
            "https://application.example.com/callback"
        )
        .is_err()
    );
    assert!(MockIdpUser::try_new("user", "User", "invalid-email").is_err());
}

#[tokio::test]
async fn discovery_and_jwks_describe_the_signed_fixture() {
    let app = mock_router();
    let discovery = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/.well-known/openid-configuration")
                .body(Body::empty())
                .expect("discovery request"),
        )
        .await
        .expect("discovery response");
    assert_eq!(discovery.status(), StatusCode::OK);
    let discovery = json_body(discovery).await;
    assert_eq!(discovery["issuer"], MOCK_IDP_ISSUER);
    assert_eq!(discovery["jwks_uri"], format!("{MOCK_IDP_ISSUER}/jwks"));
    assert_eq!(
        discovery["id_token_signing_alg_values_supported"][0],
        "EdDSA"
    );

    let jwks = app
        .oneshot(
            Request::builder()
                .uri("/jwks")
                .body(Body::empty())
                .expect("JWKS request"),
        )
        .await
        .expect("JWKS response");
    let jwks = json_body(jwks).await;
    assert_eq!(jwks["keys"][0]["kty"], "OKP");
    assert_eq!(jwks["keys"][0]["crv"], "Ed25519");
    assert_eq!(jwks["keys"][0]["kid"], signing::KEY_ID);
}

#[tokio::test]
async fn authorization_rejects_unregistered_redirects_without_redirecting() {
    let response = mock_router()
        .oneshot(
            Request::builder()
                .uri("/auth?client_id=rullst-mock-client&redirect_uri=http%3A%2F%2F127.0.0.1%3A3999%2Fstolen&response_type=code&scope=openid")
                .body(Body::empty())
                .expect("authorization request"),
        )
        .await
        .expect("authorization response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(response.headers().get(header::LOCATION).is_none());
}

#[tokio::test]
async fn userinfo_requires_an_issued_bearer_token() {
    let response = mock_router()
        .oneshot(
            Request::builder()
                .uri("/userinfo")
                .header(header::AUTHORIZATION, "Bearer invented-token")
                .body(Body::empty())
                .expect("userinfo request"),
        )
        .await
        .expect("userinfo response");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert!(response.headers().contains_key(header::WWW_AUTHENTICATE));
}

#[tokio::test]
async fn invalid_pkce_consumes_the_one_shot_authorization_code() {
    let app = mock_router();
    let (verifier, challenge) = crate::pkce::generate_pkce();
    let authorization_uri = format!(
        "/auth?client_id={MOCK_IDP_CLIENT_ID}&redirect_uri=http%3A%2F%2F127.0.0.1%3A3000%2Fcallback&response_type=code&scope=openid&code_challenge={challenge}&code_challenge_method=S256"
    );
    let authorization = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(authorization_uri)
                .body(Body::empty())
                .expect("authorization request"),
        )
        .await
        .expect("authorization response");
    let code = authorization
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| url::Url::parse(value).ok())
        .and_then(|callback| {
            callback
                .query_pairs()
                .find(|(key, _)| key == "code")
                .map(|(_, value)| value.into_owned())
        })
        .expect("one-shot authorization code");

    for candidate in ["x".repeat(43), verifier] {
        let form = serde_urlencoded::to_string([
            ("client_id", MOCK_IDP_CLIENT_ID),
            ("client_secret", MOCK_IDP_CLIENT_SECRET),
            ("code", code.as_str()),
            ("grant_type", "authorization_code"),
            ("redirect_uri", MOCK_IDP_REDIRECT_URI),
            ("code_verifier", candidate.as_str()),
        ])
        .expect("token form");
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/token")
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from(form))
                    .expect("token request"),
            )
            .await
            .expect("token response");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(json_body(response).await["error"], "invalid_grant");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn oidc_provider_verifies_signed_token_nonce_pkce_and_replay() {
    use crate::provider::{ExchangeParams, Provider};
    use crate::providers::OidcProvider;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock IdP loopback listener");
    let address = listener.local_addr().expect("mock IdP loopback address");
    let issuer = format!("http://{address}");
    let redirect_uri = "http://127.0.0.1:39001/callback";
    let config = MockIdpConfig::try_new(&issuer, "signed-client", "signed-secret", redirect_uri)
        .expect("valid mock IdP config");
    let server = tokio::spawn(async move {
        axum::serve(listener, mock_router_with_config(config))
            .await
            .expect("serve mock IdP")
    });

    let (verifier, challenge) = crate::pkce::generate_pkce();
    let provider = OidcProvider::discover(&issuer, "signed-client", "signed-secret", redirect_uri)
        .await
        .expect("discover local mock IdP")
        .with_state("state-bound-to-browser")
        .with_pkce(challenge);
    let mut authorization_url =
        url::Url::parse(&provider.redirect_url()).expect("authorization URL");
    authorization_url
        .query_pairs_mut()
        .append_pair("nonce", "nonce-bound-to-browser");

    let authorization = NoRedirectClient::new()
        .get(authorization_url)
        .send()
        .await
        .expect("authorize against mock IdP");
    assert_eq!(authorization.status(), reqwest::StatusCode::SEE_OTHER);
    let callback = authorization
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| url::Url::parse(value).ok())
        .expect("validated callback location");
    assert_eq!(callback.as_str().split('?').next(), Some(redirect_uri));
    assert_eq!(
        callback
            .query_pairs()
            .find(|(key, _)| key == "state")
            .map(|(_, value)| value.into_owned())
            .as_deref(),
        Some("state-bound-to-browser")
    );
    let code = callback
        .query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| value.into_owned())
        .expect("authorization code");

    let user = provider
        .get_user(ExchangeParams {
            auth_code: &code,
            code_verifier: Some(&verifier),
            expected_nonce: Some("nonce-bound-to-browser"),
        })
        .await
        .expect("signed OIDC exchange");
    assert_eq!(user.id, "rullst-mock-user");
    assert_eq!(user.email.as_deref(), Some("mock@example.invalid"));
    assert_eq!(user.email_verified, Some(true));

    let replay = provider
        .get_user(ExchangeParams {
            auth_code: &code,
            code_verifier: Some(&verifier),
            expected_nonce: Some("nonce-bound-to-browser"),
        })
        .await;
    assert!(replay.is_err(), "an authorization code must be one-shot");

    server.abort();
    let cancellation = server.await.expect_err("aborted mock IdP server task");
    assert!(cancellation.is_cancelled());
}

mod rullst_connect_test_support {
    pub(super) struct NoRedirectClient(reqwest::Client);

    impl NoRedirectClient {
        pub(super) fn new() -> Self {
            Self(
                reqwest::Client::builder()
                    .redirect(reqwest::redirect::Policy::none())
                    .no_proxy()
                    .timeout(std::time::Duration::from_secs(5))
                    .build()
                    .expect("build loopback-only test client"),
            )
        }

        pub(super) fn get(&self, url: url::Url) -> reqwest::RequestBuilder {
            self.0.get(url)
        }
    }
}
