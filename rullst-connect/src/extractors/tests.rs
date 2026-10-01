use super::*;

#[test]
fn auth_callback_debug_redacts_code_and_state() {
    let callback = AuthCallback {
        code: Some("code-secret-canary".to_string()),
        state: Some("state-secret-canary".to_string()),
        error: None,
        error_description: None,
    };
    let debug = format!("{callback:?}");
    assert!(!debug.contains("code-secret-canary"), "{debug}");
    assert!(!debug.contains("state-secret-canary"), "{debug}");
    assert!(debug.contains("[REDACTED]"), "{debug}");
}

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
    assert!(callback_empty.verify_state("").is_err());
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

    let req = actix_web::test::TestRequest::with_uri("/callback?code=actix_code&state=actix_state")
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

#[test]
fn test_auth_callback_verify_state() {
    let callback_valid = AuthCallback {
        code: Some("abc".into()),
        state: Some("secret_state_123".into()),
        error: None,
        error_description: None,
    };

    assert!(callback_valid.verify_state("secret_state_123").is_ok());
    let err = callback_valid.verify_state("wrong_state_456").unwrap_err();
    assert!(err.to_string().contains("CSRF state mismatch"));

    let callback_no_state = AuthCallback {
        code: Some("abc".into()),
        state: None,
        error: None,
        error_description: None,
    };
    let err2 = callback_no_state
        .verify_state("secret_state_123")
        .unwrap_err();
    assert!(err2.to_string().contains("State missing in callback"));
}

#[cfg(feature = "axum-session")]
mod session_extractor;
