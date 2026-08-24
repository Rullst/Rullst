use super::*;
use axum::{Router, http::Request, middleware, routing::get};
use tower::ServiceExt;

#[test]
fn basic_auth_debug_output_redacts_password() {
    let credentials = NexusBasicAuth::new("ops", "unique-test-secret-42")
        .expect("test credentials should be valid");
    let output = format!("{credentials:?}");

    assert!(output.contains("ops"));
    assert!(!output.contains("unique-test-secret-42"));
}

#[test]
fn basic_auth_rejects_weak_and_placeholder_credentials() {
    assert_eq!(
        NexusBasicAuth::new("ops", "too-short").expect_err("short password must fail"),
        NexusBuildError::WeakPassword {
            minimum: MIN_NEXUS_PASSWORD_LENGTH
        }
    );
    assert_eq!(
        NexusBasicAuth::new("ops", "change_me_before_deploying")
            .expect_err("placeholder password must fail"),
        NexusBuildError::PlaceholderPassword
    );
    assert_eq!(
        NexusBasicAuth::new("your_username", "unique-test-secret-42")
            .expect_err("placeholder username must fail"),
        NexusBuildError::PlaceholderUsername
    );
}

#[test]
fn basic_credentials_require_both_exact_values() {
    let credentials = NexusBasicAuth::new("ops", "unique-test-secret-42")
        .expect("test credentials should be valid");
    let valid_header =
        base64::engine::general_purpose::STANDARD.encode("ops:unique-test-secret-42");
    let wrong_user = base64::engine::general_purpose::STANDARD.encode("bad:unique-test-secret-42");
    let wrong_password = base64::engine::general_purpose::STANDARD.encode("ops:wrong-value");

    let valid = Request::builder()
        .header(header::AUTHORIZATION, format!("Basic {valid_header}"))
        .body(Body::empty())
        .expect("valid request");
    let invalid_user = Request::builder()
        .header(header::AUTHORIZATION, format!("Basic {wrong_user}"))
        .body(Body::empty())
        .expect("valid request");
    let invalid_password = Request::builder()
        .header(header::AUTHORIZATION, format!("Basic {wrong_password}"))
        .body(Body::empty())
        .expect("valid request");

    assert!(has_valid_basic_credentials(&valid, &credentials));
    assert!(!has_valid_basic_credentials(&invalid_user, &credentials));
    assert!(!has_valid_basic_credentials(
        &invalid_password,
        &credentials
    ));
}

fn basic_header(username: &str, password: &str) -> String {
    let encoded =
        base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"));
    format!("Basic {encoded}")
}

fn protected_test_router(credentials: NexusBasicAuth) -> Router {
    Router::new()
        .route("/", get(|| async { StatusCode::OK }))
        .layer(middleware::from_fn(move |request, next| {
            let credentials = credentials.clone();
            async move { basic_auth_middleware(credentials, request, next).await }
        }))
}

fn test_request(authorization: &str, tls_verified: bool) -> Request<Body> {
    let mut request = Request::builder()
        .uri("/")
        .header(header::AUTHORIZATION, authorization)
        .body(Body::empty())
        .expect("valid test request");
    request.extensions_mut().insert(ConnectInfo(
        "192.0.2.10:443".parse::<SocketAddr>().expect("test peer"),
    ));
    if tls_verified {
        request
            .extensions_mut()
            .insert(NexusVerifiedTls::from_trusted_tls_termination());
    }
    request
}

#[tokio::test]
async fn basic_auth_requires_verified_tls() {
    let credentials = NexusBasicAuth::new("ops", "unique-test-secret-42")
        .expect("test credentials should be valid");
    let app = protected_test_router(credentials);
    let request = test_request(&basic_header("ops", "unique-test-secret-42"), false);

    let response = app.oneshot(request).await.expect("router response");
    assert_eq!(response.status(), StatusCode::UPGRADE_REQUIRED);
}

#[tokio::test]
async fn basic_auth_locks_peer_after_bounded_failures() {
    let credentials = NexusBasicAuth::new("ops", "unique-test-secret-42")
        .expect("test credentials should be valid");
    let app = protected_test_router(credentials);

    for attempt in 1..NEXUS_BASIC_AUTH_MAX_FAILURES {
        let request = test_request(&basic_header("ops", "wrong-password-value"), true);
        let response = app.clone().oneshot(request).await.expect("router response");
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "attempt {attempt} should not lock early"
        );
    }

    let locking_request = test_request(&basic_header("ops", "wrong-password-value"), true);
    let locking_response = app
        .clone()
        .oneshot(locking_request)
        .await
        .expect("router response");
    assert_eq!(locking_response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(locking_response.headers().contains_key(header::RETRY_AFTER));

    let valid_request = test_request(&basic_header("ops", "unique-test-secret-42"), true);
    let locked_response = app.oneshot(valid_request).await.expect("router response");
    assert_eq!(locked_response.status(), StatusCode::TOO_MANY_REQUESTS);
}
