use super::*;
use crate::extractors::begin_oidc_session;
use crate::providers::MockProvider;
use crate::user::ConnectUser;
use axum::body::Body;
use secrecy::SecretString;
use std::sync::Arc;
use tower_sessions::{MemoryStore, Session};

const FORM: &str = "application/x-www-form-urlencoded";

fn provider() -> MockProvider {
    MockProvider::new(
        ConnectUser {
            id: "offline-user".to_string(),
            name: "Offline User".to_string(),
            email: None,
            avatar_url: None,
            email_verified: None,
            raw_data: serde_json::json!({}),
            access_token: SecretString::from("offline-token".to_string()),
            refresh_token: None,
            expires_in: None,
        },
        "https://appleid.example/auth/authorize",
    )
}

/// Starts an OIDC challenge and returns its session, state and nonce.
async fn started() -> (Session, String, String) {
    let session = Session::new(None, Arc::new(MemoryStore::default()), None);
    let authorization = begin_oidc_session(&session, &provider())
        .await
        .expect("OIDC challenge generation must succeed");
    let url = url::Url::parse(authorization.url()).expect("authorization URL must parse");
    let value = |key: &str| {
        url.query_pairs()
            .find_map(|(candidate, value)| (candidate == key).then(|| value.into_owned()))
            .expect("authorization URL parameter")
    };
    (session.clone(), value("state"), value("nonce"))
}

async fn extract(
    session: Option<&Session>,
    method: Method,
    uri: &str,
    content_type: &str,
    body: String,
) -> Result<AuthSessionForm, StatusCode> {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, content_type)
        .body(Body::from(body))
        .expect("test request must be valid");
    if let Some(session) = session {
        request.extensions_mut().insert(session.clone());
    }
    AuthSessionForm::from_request(request, &())
        .await
        .map_err(|response| response.status())
}

async fn post(session: &Session, body: String) -> Result<AuthSessionForm, StatusCode> {
    extract(Some(session), Method::POST, "/callback", FORM, body).await
}

fn apple_body(state: &str) -> String {
    serde_urlencoded::to_string([
        ("code", "authorization-code"),
        ("state", state),
        ("id_token", "header.payload.signature"),
        (
            "user",
            r#"{"name":{"firstName":"Ana"},"email":"ana@example.invalid"}"#,
        ),
    ])
    .expect("form body")
}

#[tokio::test]
async fn apple_style_form_post_consumes_the_stored_challenge() {
    let (session, state, nonce) = started().await;
    let callback = extract(
        Some(&session),
        Method::POST,
        "/callback",
        "application/x-www-form-urlencoded; charset=UTF-8",
        apple_body(&state),
    )
    .await
    .expect("matching form callback must pass");

    let params = callback.exchange_params().expect("exchange parameters");
    assert_eq!(params.auth_code, "authorization-code");
    assert_eq!(params.expected_nonce, Some(nonce.as_str()));
    assert_eq!(params.code_verifier.map(str::len), Some(64));
    let debug = format!("{callback:?}");
    assert!(!debug.contains("authorization-code"));
    assert!(!debug.contains(&nonce));
    assert_eq!(
        callback.auth_session().expected_nonce(),
        Some(nonce.as_str())
    );

    let replay = post(&session, apple_body(&state)).await;
    assert_eq!(
        replay.expect_err("replay must fail"),
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn state_mismatch_is_rejected_and_consumes_the_challenge() {
    let (session, state, _) = started().await;
    let rejected = post(&session, apple_body("attacker-state")).await;
    assert_eq!(rejected.expect_err("mismatch"), StatusCode::BAD_REQUEST);

    let later = post(&session, apple_body(&state)).await;
    assert_eq!(
        later.expect_err("consumed challenge must not recover"),
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn missing_or_duplicated_state_is_rejected() {
    let (session, _, _) = started().await;
    let missing = post(&session, "code=authorization-code".to_string()).await;
    assert_eq!(missing.expect_err("missing state"), StatusCode::BAD_REQUEST);

    let (session, state, _) = started().await;
    let duplicated = post(&session, format!("code=c&state={state}&state={state}")).await;
    assert_eq!(duplicated.expect_err("ambiguous"), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn the_query_string_is_not_read() {
    let (session, state, _) = started().await;
    let rejected = extract(
        Some(&session),
        Method::POST,
        &format!("/callback?code=authorization-code&state={state}"),
        FORM,
        String::new(),
    )
    .await;
    assert_eq!(
        rejected.expect_err("body has no state"),
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn invalid_requests_are_rejected_before_the_challenge_is_consumed() {
    let (session, state, _) = started().await;
    let oversized = format!("{}&pad={}", apple_body(&state), "a".repeat(16 * 1024));
    for (method, content_type, body, status) in [
        (
            Method::GET,
            FORM,
            apple_body(&state),
            StatusCode::METHOD_NOT_ALLOWED,
        ),
        (
            Method::POST,
            "application/json",
            apple_body(&state),
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
        ),
        (Method::POST, FORM, oversized, StatusCode::PAYLOAD_TOO_LARGE),
    ] {
        let rejected = extract(Some(&session), method, "/callback", content_type, body).await;
        assert_eq!(rejected.expect_err("invalid request"), status);
    }

    post(&session, apple_body(&state))
        .await
        .expect("rejected requests did not consume the challenge");
}

#[tokio::test]
async fn missing_session_layer_is_a_server_error() {
    let rejected = extract(None, Method::POST, "/callback", FORM, apple_body("state")).await;
    assert_eq!(
        rejected.expect_err("missing layer"),
        StatusCode::INTERNAL_SERVER_ERROR
    );
}
