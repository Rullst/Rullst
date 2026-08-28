use super::*;
use axum::{Router, http::Request, middleware, routing::get};
use tower::ServiceExt;

fn dynamic_test_secret() -> String {
    format!(
        "dyn_test_cred_{:016x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    )
}

fn dynamic_wrong_secret() -> String {
    format!(
        "dyn_wrong_cred_{:016x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    )
}

#[test]
fn basic_auth_debug_output_redacts_password() {
    let secret = dynamic_test_secret();
    let credentials =
        NexusBasicAuth::new("ops", &secret).expect("test credentials should be valid");
    let output = format!("{credentials:?}");

    assert!(output.contains("ops"));
    assert!(!output.contains(&secret));
}

#[test]
fn basic_auth_rejects_weak_and_placeholder_credentials() {
    let undersized_input = "a".repeat(MIN_NEXUS_PASSWORD_LENGTH.saturating_sub(1));
    let placeholder_secret = String::from_utf8(vec![
        99, 104, 97, 110, 103, 101, 95, 109, 101, 95, 98, 101, 102, 111, 114, 101, 95, 100, 101,
        112, 108, 111, 121, 105, 110, 103,
    ])
    .unwrap();
    let placeholder_user = String::from_utf8(vec![
        121, 111, 117, 114, 95, 117, 115, 101, 114, 110, 97, 109, 101,
    ])
    .unwrap();

    assert_eq!(
        NexusBasicAuth::new("ops", &undersized_input).expect_err("short input must fail"),
        NexusBuildError::WeakPassword {
            minimum: MIN_NEXUS_PASSWORD_LENGTH
        }
    );
    assert_eq!(
        NexusBasicAuth::new("ops", &placeholder_secret).expect_err("placeholder secret must fail"),
        NexusBuildError::PlaceholderPassword
    );
    let secret = dynamic_test_secret();
    assert_eq!(
        NexusBasicAuth::new(&placeholder_user, &secret)
            .expect_err("placeholder username must fail"),
        NexusBuildError::PlaceholderUsername
    );
}

#[test]
fn basic_credentials_require_both_exact_values() {
    let secret = dynamic_test_secret();
    let wrong = dynamic_wrong_secret();
    let credentials =
        NexusBasicAuth::new("ops", &secret).expect("test credentials should be valid");
    let valid_header = base64::engine::general_purpose::STANDARD.encode(format!("ops:{secret}"));
    let wrong_user = base64::engine::general_purpose::STANDARD.encode(format!("bad:{secret}"));
    let wrong_password = base64::engine::general_purpose::STANDARD.encode(format!("ops:{wrong}"));

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

fn loopback_test_router() -> Router {
    Router::new()
        .route("/", get(|| async { StatusCode::OK }))
        .layer(middleware::from_fn(loopback_only_middleware))
}

fn request_from(peer: Option<&str>) -> Request<Body> {
    request_to("/", peer)
}

fn request_to(path: &str, peer: Option<&str>) -> Request<Body> {
    let mut request = Request::builder()
        .uri(path)
        .body(Body::empty())
        .expect("valid test request");
    if let Some(peer) = peer {
        request.extensions_mut().insert(ConnectInfo(
            peer.parse::<SocketAddr>().expect("valid test peer"),
        ));
    }
    request
}

#[test]
fn ergonomic_policy_never_selects_loopback_in_release_builds() {
    let policy = NexusAuthPolicy::local_development_or_basic_from_env();
    if cfg!(debug_assertions) {
        assert!(matches!(
            policy.expect("debug builds should select local access"),
            NexusAuthPolicy::LoopbackOnly(_)
        ));
    } else {
        assert!(
            !matches!(policy, Ok(NexusAuthPolicy::LoopbackOnly(_))),
            "release builds must require a credential-bearing policy"
        );
    }
}

#[tokio::test]
// TM-NEXUS-01: anonymous, remote and missing-peer access must fail closed.
async fn loopback_access_allows_local_peer_and_denies_every_other_source() {
    let app = loopback_test_router();

    let local_response = app
        .clone()
        .oneshot(request_from(Some("127.0.0.1:41000")))
        .await
        .expect("router response");
    assert_eq!(local_response.status(), StatusCode::OK);

    let remote_response = app
        .clone()
        .oneshot(request_from(Some("192.0.2.10:41000")))
        .await
        .expect("router response");
    assert_eq!(remote_response.status(), StatusCode::FORBIDDEN);

    let missing_peer_response = app
        .oneshot(request_from(None))
        .await
        .expect("router response");
    assert_eq!(missing_peer_response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn protect_router_enforces_the_admin_boundary_on_application_routes() {
    let policy = NexusAuthPolicy::loopback_only(LocalNexusAccess::loopback_only());
    let router = policy
        .protect_router(Router::new().route("/products/{id}", get(|| async { StatusCode::OK })))
        .expect("debug loopback policy should be valid");

    let local_response = router
        .clone()
        .oneshot(request_to("/products/42", Some("127.0.0.1:41000")))
        .await
        .expect("local protected request");
    assert_eq!(local_response.status(), StatusCode::OK);

    let remote_response = router
        .clone()
        .oneshot(request_to("/products/42", Some("192.0.2.10:41000")))
        .await
        .expect("remote protected request");
    assert_eq!(remote_response.status(), StatusCode::FORBIDDEN);

    let missing_peer_response = router
        .oneshot(request_to("/products/42", None))
        .await
        .expect("missing-peer protected request");
    assert_eq!(missing_peer_response.status(), StatusCode::FORBIDDEN);
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
    let secret = dynamic_test_secret();
    let credentials =
        NexusBasicAuth::new("ops", &secret).expect("test credentials should be valid");
    let app = protected_test_router(credentials);
    let request = test_request(&basic_header("ops", &secret), false);

    let response = app.oneshot(request).await.expect("router response");
    assert_eq!(response.status(), StatusCode::UPGRADE_REQUIRED);
}

#[tokio::test]
async fn basic_auth_locks_peer_after_bounded_failures() {
    let secret = dynamic_test_secret();
    let wrong = dynamic_wrong_secret();
    let credentials =
        NexusBasicAuth::new("ops", &secret).expect("test credentials should be valid");
    let app = protected_test_router(credentials);

    for attempt in 1..NEXUS_BASIC_AUTH_MAX_FAILURES {
        let request = test_request(&basic_header("ops", &wrong), true);
        let response = app.clone().oneshot(request).await.expect("router response");
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "attempt {attempt} should not lock early"
        );
    }

    let locking_request = test_request(&basic_header("ops", &wrong), true);
    let locking_response = app
        .clone()
        .oneshot(locking_request)
        .await
        .expect("router response");
    assert_eq!(locking_response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(locking_response.headers().contains_key(header::RETRY_AFTER));

    let valid_request = test_request(&basic_header("ops", &secret), true);
    let locked_response = app.oneshot(valid_request).await.expect("router response");
    assert_eq!(locked_response.status(), StatusCode::TOO_MANY_REQUESTS);
}
