use super::*;
use crate::providers::MockProvider;
use crate::user::ConnectUser;
use axum::http::StatusCode;
use secrecy::SecretString;
use std::sync::Arc;
use tower_sessions::MemoryStore;

fn provider(url: &str) -> MockProvider {
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
        url,
    )
}

fn new_session() -> Session {
    Session::new(None, Arc::new(MemoryStore::default()), None)
}

async fn extract(session: Session, uri: &str) -> Result<AuthSession, StatusCode> {
    let mut request = axum::http::Request::builder()
        .uri(uri)
        .body(())
        .expect("test request URI must be valid");
    request.extensions_mut().insert(session);
    let (mut parts, _) = request.into_parts();
    AuthSession::from_request_parts(&mut parts, &())
        .await
        .map_err(|response| response.status())
}

fn query_value(url: &url::Url, key: &str) -> String {
    url.query_pairs()
        .find_map(|(candidate, value)| (candidate == key).then(|| value.into_owned()))
        .unwrap_or_else(|| panic!("authorization URL must contain {key}"))
}

#[tokio::test]
// TM-AUTH-04: state, nonce and PKCE are server-bound, redacted and single-use.
async fn oidc_session_round_trip_is_sequentially_single_use_and_keeps_secrets_server_side() {
    let session = new_session();
    let authorization = begin_oidc_session(
        &session,
        &provider("https://idp.example/authorize?client_id=test"),
    )
    .await
    .expect("OIDC challenge generation must succeed");
    let url = url::Url::parse(authorization.url()).expect("authorization URL must parse");
    let state = query_value(&url, "state");
    let nonce = query_value(&url, "nonce");

    assert_eq!(query_value(&url, "code_challenge_method"), "S256");
    assert_eq!(query_value(&url, "code_challenge").len(), 43);
    assert!(!authorization.url().contains("offline-token"));
    assert!(!format!("{authorization:?}").contains(&state));
    assert!(!format!("{authorization:?}").contains(&nonce));

    let callback_uri = format!("/callback?code=authorization-code&state={state}");
    let auth_session = extract(session.clone(), &callback_uri)
        .await
        .expect("matching OIDC callback must pass");
    let params = auth_session
        .exchange_params()
        .expect("validated callback must create exchange parameters");

    assert_eq!(params.auth_code, "authorization-code");
    assert_eq!(params.expected_nonce, Some(nonce.as_str()));
    assert_eq!(params.code_verifier.map(str::len), Some(64));
    assert_eq!(auth_session.expected_nonce(), Some(nonce.as_str()));
    assert_eq!(auth_session.code_verifier().map(str::len), Some(64));
    let debug = format!("{auth_session:?}");
    assert!(!debug.contains("authorization-code"));
    assert!(!debug.contains(&nonce));
    assert!(!debug.contains(params.code_verifier.expect("PKCE verifier must exist")));

    let replay = extract(session, &callback_uri).await;
    assert_eq!(
        replay.expect_err("callback replay must fail"),
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn oauth_session_uses_pkce_without_an_oidc_nonce() {
    let session = new_session();
    let authorization = begin_oauth_session(&session, &provider("https://idp.example/authorize"))
        .await
        .expect("OAuth challenge generation must succeed");
    let url = url::Url::parse(authorization.url()).expect("authorization URL must parse");
    let state = query_value(&url, "state");

    assert!(url.query_pairs().all(|(key, _)| key != "nonce"));
    let auth_session = extract(
        session,
        &format!("/callback?code=authorization-code&state={state}"),
    )
    .await
    .expect("matching OAuth callback must pass");

    assert_eq!(auth_session.expected_nonce(), None);
    assert_eq!(auth_session.code_verifier().map(str::len), Some(64));
}

#[tokio::test]
async fn mismatched_or_missing_state_consumes_the_challenge() {
    for invalid_uri in [
        "/callback?code=authorization-code&state=wrong",
        "/callback?code=authorization-code",
    ] {
        let session = new_session();
        let authorization =
            begin_oidc_session(&session, &provider("https://idp.example/authorize"))
                .await
                .expect("OIDC challenge generation must succeed");
        let url = url::Url::parse(authorization.url()).expect("authorization URL must parse");
        let state = query_value(&url, "state");

        let rejected = extract(session.clone(), invalid_uri).await;
        assert_eq!(
            rejected.expect_err("invalid state must fail"),
            StatusCode::BAD_REQUEST
        );

        let later_match = extract(
            session,
            &format!("/callback?code=authorization-code&state={state}"),
        )
        .await;
        assert_eq!(
            later_match.expect_err("consumed challenge must not recover"),
            StatusCode::BAD_REQUEST
        );
    }
}

#[tokio::test]
async fn expired_challenge_fails_closed_and_is_consumed() {
    let session = new_session();
    session
        .insert(
            CHALLENGE_KEY,
            StoredChallenge {
                state: "expired-state".to_string(),
                nonce: Some("expired-nonce".to_string()),
                code_verifier: "expired-verifier".to_string(),
                expires_at_unix_seconds: 0,
            },
        )
        .await
        .expect("test challenge must be stored");

    let first = extract(
        session.clone(),
        "/callback?code=authorization-code&state=expired-state",
    )
    .await;
    assert_eq!(
        first.expect_err("expired challenge must fail"),
        StatusCode::BAD_REQUEST
    );
    let replay = extract(
        session,
        "/callback?code=authorization-code&state=expired-state",
    )
    .await;
    assert_eq!(
        replay.expect_err("expired challenge must be consumed"),
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn starting_a_flow_replaces_legacy_and_previous_challenges() {
    let session = new_session();
    session
        .insert(LEGACY_STATE_KEY, "legacy-state".to_string())
        .await
        .expect("legacy state must be stored");
    let first = begin_oauth_session(&session, &provider("https://idp.example/authorize"))
        .await
        .expect("first challenge must succeed");
    let first_state = query_value(
        &url::Url::parse(first.url()).expect("first URL must parse"),
        "state",
    );
    let second = begin_oauth_session(&session, &provider("https://idp.example/authorize"))
        .await
        .expect("replacement challenge must succeed");
    let second_state = query_value(
        &url::Url::parse(second.url()).expect("second URL must parse"),
        "state",
    );
    assert!(
        session
            .get::<String>(LEGACY_STATE_KEY)
            .await
            .expect("legacy lookup must succeed")
            .is_none(),
        "starting a managed flow must remove the state-only fallback"
    );

    let first_callback = extract(
        session.clone(),
        &format!("/callback?code=authorization-code&state={first_state}"),
    )
    .await;
    assert_eq!(
        first_callback.expect_err("old challenge must be replaced"),
        StatusCode::BAD_REQUEST
    );
    let second_callback = extract(
        session,
        &format!("/callback?code=authorization-code&state={second_state}"),
    )
    .await;
    assert_eq!(
        second_callback.expect_err("mismatch consumed replacement"),
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn invalid_provider_authorization_url_is_rejected() {
    for invalid_url in [
        "not a provider URL",
        "javascript:alert(1)",
        "http://idp.example/authorize",
        "https://user:password@idp.example/authorize",
        "https://idp.example/authorize#fragment",
        "https://idp.example/authorize?state=attacker",
        "https://idp.example/authorize?nonce=attacker",
        "https://idp.example/authorize?code_challenge=attacker",
        "https://idp.example/authorize?code_challenge_method=plain",
    ] {
        let error = begin_oauth_session(&new_session(), &provider(invalid_url))
            .await
            .expect_err("unsafe provider URL must fail");
        assert!(matches!(
            error,
            ConnectError::InvalidConfiguration {
                field: "authorization_url",
                ..
            }
        ));
    }
}

#[tokio::test]
async fn provider_error_is_bounded_and_callback_without_code_is_rejected() {
    let session = new_session();
    let authorization = begin_oauth_session(&session, &provider("https://idp.example/authorize"))
        .await
        .expect("OAuth challenge must succeed");
    let state = query_value(
        &url::Url::parse(authorization.url()).expect("authorization URL must parse"),
        "state",
    );
    let long_error = "x".repeat(300);
    let callback = extract(
        session,
        &format!("/callback?error={long_error}&state={state}"),
    )
    .await
    .expect("matching error callback must validate state");
    let error = callback
        .exchange_params()
        .expect_err("provider error must stop token exchange")
        .to_string();
    assert!(error.len() < 220);

    let session = new_session();
    let authorization = begin_oauth_session(&session, &provider("https://idp.example/authorize"))
        .await
        .expect("OAuth challenge must succeed");
    let state = query_value(
        &url::Url::parse(authorization.url()).expect("authorization URL must parse"),
        "state",
    );
    let callback = extract(session, &format!("/callback?state={state}"))
        .await
        .expect("matching callback must validate state");
    assert!(matches!(
        callback.exchange_params(),
        Err(ConnectError::Token(_))
    ));
}
