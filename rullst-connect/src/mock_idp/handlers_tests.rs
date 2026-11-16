use super::*;
use crate::mock_idp::{MOCK_IDP_CLIENT_ID, MOCK_IDP_CLIENT_SECRET, MOCK_IDP_REDIRECT_URI};
use axum::body::{Body, to_bytes};
use axum::http::Request;
use serde_json::Value;
use tower::ServiceExt;

fn authorization(challenge: Option<&str>, method: Option<&str>) -> AuthorizationRequest {
    AuthorizationRequest {
        client_id: MOCK_IDP_CLIENT_ID.to_string(),
        redirect_uri: MOCK_IDP_REDIRECT_URI.to_string(),
        response_type: "code".to_string(),
        scope: Some("openid email".to_string()),
        state: None,
        nonce: None,
        code_challenge: challenge.map(str::to_string),
        code_challenge_method: method.map(str::to_string),
    }
}

fn token(client_id: &str, client_secret: &str) -> TokenRequest {
    TokenRequest {
        client_id: client_id.to_string(),
        client_secret: client_secret.to_string(),
        code: "rullst_mock_code_0000000000000001".to_string(),
        grant_type: "authorization_code".to_string(),
        redirect_uri: MOCK_IDP_REDIRECT_URI.to_string(),
        code_verifier: None,
    }
}

#[test]
fn request_fields_hold_at_most_512_bytes_without_controls() {
    assert_eq!(
        validate_request_fields(["a".repeat(512).as_str(), ""]),
        Ok(())
    );
    for field in ["a".repeat(513), "a\u{7}b".to_string()] {
        assert_eq!(
            validate_request_fields([field.as_str()]),
            Err("request fields exceed the bounded input policy"),
            "{field:?}"
        );
    }
}

#[test]
fn token_requests_need_both_client_credentials() {
    let config = MockIdpConfig::default();
    assert_eq!(
        validate_token_request(&config, &token(MOCK_IDP_CLIENT_ID, MOCK_IDP_CLIENT_SECRET)),
        Ok(())
    );
    for request in [
        token(MOCK_IDP_CLIENT_ID, "wrong-secret"),
        token("wrong-client", MOCK_IDP_CLIENT_SECRET),
    ] {
        assert_eq!(
            validate_token_request(&config, &request),
            Err("client authentication failed")
        );
    }
    let mut oversized = token(MOCK_IDP_CLIENT_ID, MOCK_IDP_CLIENT_SECRET);
    oversized.code = "c".repeat(513);
    assert_eq!(
        validate_token_request(&config, &oversized),
        Err("request fields exceed the bounded input policy")
    );
    assert!(secret_eq("same", "same"));
    assert!(!secret_eq("same", "other"));
    assert_ne!(hash_secret("same"), hash_secret("other"));
}

#[test]
fn pkce_challenges_are_43_url_safe_characters_with_s256() {
    let config = MockIdpConfig::default();
    let valid = format!("{}-_", "A".repeat(41));
    assert_eq!(
        validate_authorization_request(&config, &authorization(Some(&valid), Some("S256"))),
        Ok(())
    );
    assert_eq!(
        validate_authorization_request(&config, &authorization(None, None)),
        Ok(())
    );
    let rejected = Err("PKCE must use one valid S256 code challenge");
    for (challenge, method) in [
        (Some("A".repeat(42)), Some("S256")),
        (Some("A".repeat(44)), Some("S256")),
        (Some(format!("{}+", "A".repeat(42))), Some("S256")),
        (Some(valid.clone()), Some("plain")),
        (Some(valid.clone()), None),
        (None, Some("S256")),
    ] {
        assert_eq!(
            validate_authorization_request(&config, &authorization(challenge.as_deref(), method)),
            rejected,
            "{challenge:?} {method:?}"
        );
    }
}

async fn send(app: &Router, request: Request<Body>) -> (StatusCode, Value, Option<String>) {
    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("mock IdP response");
    let status = response.status();
    let location = response
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let bytes = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("bounded body");
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body, location)
}

async fn authorize(app: &Router, challenge: Option<&str>) -> String {
    let pkce = challenge
        .map(|challenge| format!("&code_challenge={challenge}&code_challenge_method=S256"))
        .unwrap_or_default();
    let uri = format!(
        "/auth?client_id={MOCK_IDP_CLIENT_ID}&redirect_uri=http%3A%2F%2F127.0.0.1%3A3000%2Fcallback&response_type=code&scope=openid{pkce}"
    );
    let (status, _, location) = send(
        app,
        Request::builder()
            .uri(uri)
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    let callback = url::Url::parse(&location.expect("redirect")).expect("callback URL");
    callback
        .query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| value.into_owned())
        .expect("authorization code")
}

async fn exchange(app: &Router, code: &str, verifier: Option<&str>) -> (StatusCode, Value) {
    let mut fields = vec![
        ("client_id", MOCK_IDP_CLIENT_ID),
        ("client_secret", MOCK_IDP_CLIENT_SECRET),
        ("code", code),
        ("grant_type", "authorization_code"),
        ("redirect_uri", MOCK_IDP_REDIRECT_URI),
    ];
    if let Some(verifier) = verifier {
        fields.push(("code_verifier", verifier));
    }
    let form = serde_urlencoded::to_string(fields).expect("token form");
    let (status, body, _) = send(
        app,
        Request::builder()
            .method("POST")
            .uri("/token")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from(form))
            .expect("token request"),
    )
    .await;
    (status, body)
}

async fn userinfo(app: &Router, access_token: &str) -> StatusCode {
    send(
        app,
        Request::builder()
            .uri("/userinfo")
            .header(header::AUTHORIZATION, format!("Bearer {access_token}"))
            .body(Body::empty())
            .expect("userinfo request"),
    )
    .await
    .0
}

#[tokio::test]
async fn earlier_codes_and_tokens_stay_valid_after_later_requests() {
    let app = router(MockIdpConfig::default());
    let first = authorize(&app, None).await;
    let second = authorize(&app, None).await;
    let (status, first_tokens) = exchange(&app, &first, None).await;
    assert_eq!(status, StatusCode::OK, "{first_tokens}");
    let (status, second_tokens) = exchange(&app, &second, None).await;
    assert_eq!(status, StatusCode::OK, "{second_tokens}");
    for tokens in [&first_tokens, &second_tokens] {
        let access_token = tokens["access_token"].as_str().expect("access token");
        assert_eq!(userinfo(&app, access_token).await, StatusCode::OK);
    }
    assert_ne!(first_tokens["access_token"], second_tokens["access_token"]);
}

#[tokio::test]
async fn a_pkce_verifier_must_have_43_to_128_allowed_characters() {
    let app = router(MockIdpConfig::default());
    // Each verifier matches its challenge, so only its form is refused.
    for verifier in ["a".repeat(42), format!("{}+", "a".repeat(42))] {
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        let code = authorize(&app, Some(&challenge)).await;
        let (status, body) = exchange(&app, &code, Some(&verifier)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{verifier}");
        assert_eq!(body["error"], "invalid_grant");
    }
    let verifier = "a".repeat(43);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let code = authorize(&app, Some(&challenge)).await;
    assert_eq!(
        exchange(&app, &code, Some(&verifier)).await.0,
        StatusCode::OK
    );
}

/// The mock IdP of `earlier_codes_…` on a clock the test sets, from a fixed
/// instant (14 January 2027).
fn manual_clock() -> (Router, Arc<std::sync::atomic::AtomicU64>) {
    let seconds = Arc::new(std::sync::atomic::AtomicU64::new(1_800_000_000));
    let app = router_with_clock(
        MockIdpConfig::default(),
        Clock::Manual(Arc::clone(&seconds)),
    );
    (app, seconds)
}

fn advance(clock: &std::sync::atomic::AtomicU64, seconds: u64) {
    clock.fetch_add(seconds, std::sync::atomic::Ordering::SeqCst);
}

#[tokio::test]
async fn an_authorization_code_expires_five_minutes_after_it_was_issued() {
    let (app, clock) = manual_clock();
    // In its last second the code is still exchanged.
    let code = authorize(&app, None).await;
    advance(&clock, 299);
    let (status, body) = exchange(&app, &code, None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    // From the 300th second, the same client and redirect URI are refused.
    let code = authorize(&app, None).await;
    advance(&clock, 300);
    let (status, body) = exchange(&app, &code, None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["error"], "invalid_grant");
    assert_eq!(
        body["error_description"],
        "the authorization code binding is invalid or expired"
    );
    // An expired code is consumed too: it cannot be retried.
    let (status, body) = exchange(&app, &code, None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["error_description"],
        "the authorization code is invalid, expired, or already consumed"
    );
}

#[tokio::test]
async fn an_access_token_answers_userinfo_for_one_hour() {
    let (app, clock) = manual_clock();
    let code = authorize(&app, None).await;
    let (status, tokens) = exchange(&app, &code, None).await;
    assert_eq!(status, StatusCode::OK, "{tokens}");
    assert_eq!(tokens["expires_in"], 3_600);
    let access_token = tokens["access_token"].as_str().expect("access token");
    advance(&clock, 3_599);
    assert_eq!(userinfo(&app, access_token).await, StatusCode::OK);
    advance(&clock, 1);
    assert_eq!(userinfo(&app, access_token).await, StatusCode::UNAUTHORIZED);
}
