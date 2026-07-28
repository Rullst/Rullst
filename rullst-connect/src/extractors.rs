use serde::Deserialize;

/// Standard OAuth2 callback query parameters.
///
/// Most web frameworks (like Axum, Actix, Leptos, Rocket) can automatically
/// deserialize URL query strings into this struct.
///
/// # Example (Axum)
/// ```rust,ignore
/// async fn auth_callback(Query(params): Query<AuthCallback>) -> impl IntoResponse {
///     if let Some(error) = params.error {
///         return format!("Auth failed: {}", error);
///     }
///     
///     let Some(code) = params.code else {
///         return "Authorization code missing".into_response();
///     };
///     // Handle token exchange...
/// }
/// ```
#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct AuthCallback {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

impl AuthCallback {
    /// Helper to verify the CSRF state parameter.
    pub fn verify_state(&self, session_state: &str) -> Result<(), crate::error::ConnectError> {
        use sha2::{Digest, Sha256};
        use subtle::ConstantTimeEq;

        match &self.state {
            Some(state) => {
                let hash_state = Sha256::digest(state.as_bytes());
                let hash_session = Sha256::digest(session_state.as_bytes());

                if bool::from(hash_state.ct_eq(&hash_session)) {
                    Ok(())
                } else {
                    Err(crate::error::ConnectError::InvalidState(
                        "CSRF state mismatch".into(),
                    ))
                }
            }
            None => Err(crate::error::ConnectError::InvalidState(
                "State missing in callback".into(),
            )),
        }
    }
}

#[cfg(feature = "axum")]
impl<S> axum::extract::FromRequestParts<S> for AuthCallback
where
    S: Send + Sync,
{
    type Rejection = axum::extract::rejection::QueryRejection;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let axum::extract::Query(callback) =
            axum::extract::Query::<AuthCallback>::from_request_parts(parts, state).await?;
        Ok(callback)
    }
}

#[cfg(feature = "actix")]
impl actix_web::FromRequest for AuthCallback {
    type Error = actix_web::Error;
    type Future = std::future::Ready<Result<Self, Self::Error>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        match actix_web::web::Query::<AuthCallback>::from_query(req.query_string()) {
            Ok(query) => std::future::ready(Ok(query.into_inner())),
            Err(e) => std::future::ready(Err(e.into())),
        }
    }
}

#[cfg(feature = "axum-session")]
#[derive(Debug, Clone)]
pub struct AuthSession {
    pub callback: AuthCallback,
}

#[cfg(feature = "axum-session")]
impl<S> axum::extract::FromRequestParts<S> for AuthSession
where
    S: Send + Sync,
{
    type Rejection = axum::response::Response;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let session = parts
            .extensions
            .get::<tower_sessions::Session>()
            .cloned()
            .ok_or_else(|| {
                axum::response::IntoResponse::into_response((
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "Missing tower-sessions extension",
                ))
            })?;

        let axum::extract::Query(callback) =
            axum::extract::Query::<AuthCallback>::from_request_parts(parts, state)
                .await
                .map_err(axum::response::IntoResponse::into_response)?;

        let state_param = callback.state.as_ref().ok_or_else(|| {
            axum::response::IntoResponse::into_response((
                axum::http::StatusCode::BAD_REQUEST,
                "Missing CSRF state parameter",
            ))
        })?;

        use subtle::ConstantTimeEq;
        let session_state: Option<String> = session.remove("oauth_state").await.unwrap_or(None);
        if let Some(saved) = session_state
            && state_param.len() == saved.len()
            && bool::from(state_param.as_bytes().ct_eq(saved.as_bytes()))
        {
            // Valid!
            Ok(Self { callback })
        } else {
            Err(axum::response::IntoResponse::into_response((
                axum::http::StatusCode::BAD_REQUEST,
                "CSRF state mismatch",
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_callback_success_deserialization() {
        let query = "code=auth_code_123&state=state_xyz";
        let callback: AuthCallback =
            serde_urlencoded::from_str(query).expect("Failed to deserialize valid query string");

        assert_eq!(callback.code.as_deref(), Some("auth_code_123"));
        assert_eq!(callback.state.as_deref(), Some("state_xyz"));
        assert_eq!(callback.error, None);
        assert_eq!(callback.error_description, None);
    }

    #[test]
    fn test_auth_callback_error_deserialization() {
        let query = "error=access_denied&error_description=User%20denied%20access&state=state_xyz";
        let callback: AuthCallback =
            serde_urlencoded::from_str(query).expect("Failed to deserialize error query string");

        assert_eq!(callback.code, None);
        assert_eq!(callback.state.as_deref(), Some("state_xyz"));
        assert_eq!(callback.error.as_deref(), Some("access_denied"));
        assert_eq!(
            callback.error_description.as_deref(),
            Some("User denied access")
        );
    }

    #[test]
    fn test_auth_callback_empty_deserialization() {
        let query = "";
        let callback: AuthCallback =
            serde_urlencoded::from_str(query).expect("Failed to deserialize empty query string");

        assert_eq!(callback.code, None);
        assert_eq!(callback.state, None);
        assert_eq!(callback.error, None);
        assert_eq!(callback.error_description, None);
    }

    #[test]
    fn test_verify_state() {
        // 1. Valid state matching
        let callback_valid = AuthCallback {
            code: None,
            state: Some("state_123".to_owned()),
            error: None,
            error_description: None,
        };
        assert!(callback_valid.verify_state("state_123").is_ok());

        // 2. State mismatch
        let res_mismatch = callback_valid.verify_state("state_xyz");
        assert!(res_mismatch.is_err());
        match res_mismatch.unwrap_err() {
            crate::error::ConnectError::InvalidState(msg) => {
                assert_eq!(msg, "CSRF state mismatch");
            }
            _ => panic!("Expected ConnectError::InvalidState"),
        }

        // 2.5 State mismatch (different length)
        let res_mismatch_len = callback_valid.verify_state("state_12345");
        assert!(res_mismatch_len.is_err());
        match res_mismatch_len.unwrap_err() {
            crate::error::ConnectError::InvalidState(msg) => {
                assert_eq!(msg, "CSRF state mismatch");
            }
            _ => panic!("Expected ConnectError::InvalidState"),
        }

        // 3. State missing
        let callback_missing = AuthCallback {
            code: None,
            state: None,
            error: None,
            error_description: None,
        };
        let res_missing = callback_missing.verify_state("state_123");
        assert!(res_missing.is_err());
        match res_missing.unwrap_err() {
            crate::error::ConnectError::InvalidState(msg) => {
                assert_eq!(msg, "State missing in callback");
            }
            _ => panic!("Expected ConnectError::InvalidState"),
        }

        // 4. Empty state string edge cases
        let callback_empty = AuthCallback {
            code: None,
            state: Some("".to_owned()),
            error: None,
            error_description: None,
        };
        assert!(callback_empty.verify_state("").is_ok());
        assert!(callback_empty.verify_state("not_empty").is_err());
        assert!(callback_valid.verify_state("").is_err());

        // 5. Verify the return value on success is exactly Ok(())
        //    Kills the mutant: `replace verify_state -> Result<(), ConnectError> with Ok(())`
        //    because if it always returned Ok(()) the mismatch checks above would fail.
        //    This additional check makes the success path explicit.
        assert!(
            callback_valid.verify_state("state_123").is_ok(),
            "verify_state must return Ok(()) on match"
        );

        // 6. Kills `replace == with !=` on the length check:
        //    When lengths differ, state comparison must fail even if bytes overlap.
        let callback_prefix = AuthCallback {
            code: None,
            state: Some("state_1".to_owned()), // shorter than "state_123"
            error: None,
            error_description: None,
        };
        assert!(
            callback_prefix.verify_state("state_123").is_err(),
            "verify_state must fail when state is a prefix of session_state"
        );
        assert!(
            callback_valid.verify_state("state_1").is_err(),
            "verify_state must fail when session_state is a prefix of state"
        );

        // 7. Kills `replace && with ||` on both conditions:
        //    Both the length check AND the constant-time comparison must hold.
        //    Correct length but wrong content → must fail.
        let same_len_wrong = AuthCallback {
            code: None,
            state: Some("state_000".to_owned()), // same length as "state_123", different content
            error: None,
            error_description: None,
        };
        assert!(
            same_len_wrong.verify_state("state_123").is_err(),
            "verify_state must fail when state is same length but different content"
        );
    }

    #[test]
    fn test_verify_state_missing_state_edge_case() {
        let callback_missing = AuthCallback {
            code: Some("some_code".to_owned()),
            state: None,
            error: None,
            error_description: None,
        };

        let err = callback_missing
            .verify_state("session_state_123")
            .unwrap_err();
        assert!(matches!(err, crate::error::ConnectError::InvalidState(_)));
        if let crate::error::ConnectError::InvalidState(msg) = err {
            assert_eq!(msg, "State missing in callback");
        }
    }

    #[cfg(feature = "actix")]
    #[tokio::test]
    async fn test_actix_extractor() {
        use actix_web::FromRequest;

        let req =
            actix_web::test::TestRequest::with_uri("/callback?code=actix_code&state=actix_state")
                .to_http_request();
        let payload = &mut actix_web::dev::Payload::None;
        let callback = AuthCallback::from_request(&req, payload).await.unwrap();
        assert_eq!(callback.code.as_deref(), Some("actix_code"));
        assert_eq!(callback.state.as_deref(), Some("actix_state"));

        // Test error case (invalid query format)
        let req_err =
            actix_web::test::TestRequest::with_uri("/callback?code=a&code=b").to_http_request();
        let res_err = AuthCallback::from_request(&req_err, payload).await;
        assert!(res_err.is_err());

        // Kills mutant on L83: `replace from_request -> Self::Future with Default::default()`.
        // If the impl returned Default::default() (always Ok with empty fields), the
        // assertion below would fail because code would be None instead of the real value.
        let req_vals =
            actix_web::test::TestRequest::with_uri("/callback?code=distinct_code&state=dist_state")
                .to_http_request();
        let cb = AuthCallback::from_request(&req_vals, payload)
            .await
            .unwrap();
        assert_eq!(
            cb.code.as_deref(),
            Some("distinct_code"),
            "from_request must parse query, not return default"
        );
        assert_eq!(
            cb.state.as_deref(),
            Some("dist_state"),
            "from_request must parse query, not return default"
        );
    }

    #[cfg(feature = "axum")]
    #[tokio::test]
    async fn test_axum_extractor() {
        use axum::extract::FromRequestParts;

        let req = axum::http::Request::builder()
            .uri("/callback?code=axum_code&state=axum_state")
            .body(())
            .unwrap();

        let (mut parts, _) = req.into_parts();
        let callback = AuthCallback::from_request_parts(&mut parts, &())
            .await
            .unwrap();

        assert_eq!(callback.code.as_deref(), Some("axum_code"));
        assert_eq!(callback.state.as_deref(), Some("axum_state"));
    }

    /// Kills mutant on extractors.rs L68:
    /// `replace from_request_parts (AuthCallback axum) -> Ok(Default::default())`.
    /// If the impl always returned the default struct, a request with actual
    /// query params would succeed but with empty values — this test catches that.
    #[cfg(feature = "axum")]
    #[tokio::test]
    async fn test_axum_extractor_values_are_from_query() {
        use axum::extract::FromRequestParts;

        // Verify that the extractor actually reads from the query string and
        // doesn't just return a default/empty value.
        let req = axum::http::Request::builder()
            .uri("/callback?code=real_code_value&state=real_state_value&error=real_error")
            .body(())
            .unwrap();

        let (mut parts, _) = req.into_parts();
        let callback = AuthCallback::from_request_parts(&mut parts, &())
            .await
            .unwrap();

        // If the mutant returned Default::default(), these would be None.
        assert_eq!(
            callback.code.as_deref(),
            Some("real_code_value"),
            "code must come from the query string, not default"
        );
        assert_eq!(
            callback.state.as_deref(),
            Some("real_state_value"),
            "state must come from the query string, not default"
        );
        assert_eq!(
            callback.error.as_deref(),
            Some("real_error"),
            "error must come from the query string, not default"
        );
    }

    #[cfg(feature = "axum-session")]
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

    #[cfg(feature = "axum-session")]
    #[tokio::test]
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

    #[cfg(feature = "axum-session")]
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

    #[cfg(feature = "axum-session")]
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

    #[cfg(feature = "axum-session")]
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
    #[cfg(feature = "axum-session")]
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
    #[cfg(feature = "axum-session")]
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
}
