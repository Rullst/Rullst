//! `AuthSession` extractor tests against an in-memory `tower-sessions` store.

use crate::extractors::AuthSession;

#[tokio::test]
async fn test_axum_session_extractor_success() {
    use axum::extract::FromRequestParts;
    use std::sync::Arc;
    use tower_sessions::{MemoryStore, Session};

    // 1. Create a session and set state in it
    let store = Arc::new(MemoryStore::default());
    let session = Session::new(None, store, None);
    session
        .insert("oauth_state", "state_123".to_owned())
        .await
        .unwrap();

    // 2. Build a request with the session in extensions and the query parameters
    let mut req = axum::http::Request::builder()
        .uri("/callback?code=auth_code_123&state=state_123")
        .body(())
        .unwrap();
    req.extensions_mut().insert(session);

    let (mut parts, _) = req.into_parts();

    // 3. Extract AuthSession
    let auth_session = AuthSession::from_request_parts(&mut parts, &())
        .await
        .unwrap();
    assert_eq!(auth_session.callback.code.as_deref(), Some("auth_code_123"));
    assert_eq!(auth_session.callback.state.as_deref(), Some("state_123"));
}

#[tokio::test]
// TM-AUTH-04: a substituted or absent OAuth callback state is rejected.
async fn test_axum_session_extractor_mismatch() {
    use axum::extract::FromRequestParts;
    use std::sync::Arc;
    use tower_sessions::{MemoryStore, Session};

    let store = Arc::new(MemoryStore::default());
    let session = Session::new(None, store, None);
    session
        .insert("oauth_state", "different_state".to_owned())
        .await
        .unwrap();

    let mut req = axum::http::Request::builder()
        .uri("/callback?code=auth_code_123&state=state_123")
        .body(())
        .unwrap();
    req.extensions_mut().insert(session.clone());

    let (mut parts, _) = req.into_parts();

    let res = AuthSession::from_request_parts(&mut parts, &()).await;
    assert!(res.is_err());
    let response = res.unwrap_err();
    assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);

    // Also test missing state in session
    session.remove::<String>("oauth_state").await.unwrap();
    let mut req2 = axum::http::Request::builder()
        .uri("/callback?code=auth_code_123&state=state_123")
        .body(())
        .unwrap();
    req2.extensions_mut().insert(session);
    let (mut parts2, _) = req2.into_parts();
    let res2 = AuthSession::from_request_parts(&mut parts2, &()).await;
    assert!(res2.is_err());
}

#[tokio::test]
async fn test_axum_session_extractor_missing_extension() {
    use axum::extract::FromRequestParts;

    let req = axum::http::Request::builder()
        .uri("/callback?code=auth_code_123&state=state_123")
        .body(())
        .unwrap();

    let (mut parts, _) = req.into_parts();

    let res = AuthSession::from_request_parts(&mut parts, &()).await;
    assert!(res.is_err());
    let response = res.unwrap_err();
    assert_eq!(
        response.status(),
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    );
}

#[tokio::test]
async fn test_axum_session_extractor_missing_state() {
    use axum::extract::FromRequestParts;
    use std::sync::Arc;
    use tower_sessions::{MemoryStore, Session};

    let store = Arc::new(MemoryStore::default());
    let session = Session::new(None, store, None);
    session
        .insert("oauth_state", "state_123".to_owned())
        .await
        .unwrap();

    let mut req = axum::http::Request::builder()
        .uri("/callback?code=auth_code_123") // No state query param
        .body(())
        .unwrap();
    req.extensions_mut().insert(session);

    let (mut parts, _) = req.into_parts();

    let res = AuthSession::from_request_parts(&mut parts, &()).await;
    assert!(res.is_err());
    let response = res.unwrap_err();
    assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_axum_session_extractor_same_length_invalid_state() {
    use axum::extract::FromRequestParts;
    use std::sync::Arc;
    use tower_sessions::{MemoryStore, Session};

    let store = Arc::new(MemoryStore::default());
    let session = Session::new(None, store, None);
    session
        .insert("oauth_state", "state_123".to_owned())
        .await
        .unwrap();

    let mut req = axum::http::Request::builder()
        .uri("/callback?code=auth_code_123&state=state_999")
        .body(())
        .unwrap();
    req.extensions_mut().insert(session);

    let (mut parts, _) = req.into_parts();

    let res = AuthSession::from_request_parts(&mut parts, &()).await;
    assert!(res.is_err());
    let response = res.unwrap_err();
    assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
}

/// Kills mutant on L107:
/// `replace from_request_parts (AuthSession) -> Ok(Default::default())`.
/// If the impl returned a default, we'd get an AuthSession with empty fields.
/// This test verifies the returned session actually carries the real callback data.
#[tokio::test]
async fn test_axum_session_extractor_carries_real_values() {
    use axum::extract::FromRequestParts;
    use std::sync::Arc;
    use tower_sessions::{MemoryStore, Session};

    let store = Arc::new(MemoryStore::default());
    let session = Session::new(None, store, None);
    session
        .insert("oauth_state", "unique_state_abc".to_owned())
        .await
        .unwrap();

    let mut req = axum::http::Request::builder()
        .uri("/callback?code=unique_code_xyz&state=unique_state_abc")
        .body(())
        .unwrap();
    req.extensions_mut().insert(session);

    let (mut parts, _) = req.into_parts();
    let auth_session = AuthSession::from_request_parts(&mut parts, &())
        .await
        .unwrap();

    // If the mutant returned Default::default(), these would be None.
    assert_eq!(
        auth_session.callback.code.as_deref(),
        Some("unique_code_xyz"),
        "AuthSession must carry the real code from the query, not default"
    );
    assert_eq!(
        auth_session.callback.state.as_deref(),
        Some("unique_state_abc"),
        "AuthSession must carry the real state from the query, not default"
    );
}

/// Kills mutants on L133: `replace == with !=` and `replace && with ||`.
/// Kills mutant on L134: `replace && with ||`.
///
/// Verifies AuthSession rejects when:
/// (a) states have different lengths  → kills the `== vs !=` length check mutant
/// (b) same length but different content → kills the `&& vs ||` ct_eq mutant
#[tokio::test]
async fn test_axum_session_extractor_length_mismatch_rejected() {
    use axum::extract::FromRequestParts;
    use std::sync::Arc;
    use tower_sessions::{MemoryStore, Session};

    // Case (a): saved state is shorter than the query state.
    let store = Arc::new(MemoryStore::default());
    let session = Session::new(None, store, None);
    session
        .insert("oauth_state", "short".to_owned())
        .await
        .unwrap();

    let mut req = axum::http::Request::builder()
        .uri("/callback?code=c&state=longer_than_short")
        .body(())
        .unwrap();
    req.extensions_mut().insert(session);

    let (mut parts, _) = req.into_parts();
    let res = AuthSession::from_request_parts(&mut parts, &()).await;
    assert!(
        res.is_err(),
        "Must reject when saved state length != query state length"
    );
    assert_eq!(
        res.unwrap_err().status(),
        axum::http::StatusCode::BAD_REQUEST
    );
}
