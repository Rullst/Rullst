use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
    routing::post,
};
use rullst_core::{
    config::{Environment, SecurityConfig},
    security::{
        MachineEndpoint, MachineEndpointError, MachineEndpointPolicy, MachineRequestVerifier,
        apply_security_baseline_with_machine_endpoints,
    },
};
use tower::ServiceExt;

const TOKEN: &str = "test_machine_0123456789_ABCDEFGHIJKLMNOPQRSTUVWXYZ";

fn policy() -> MachineEndpointPolicy {
    MachineEndpointPolicy::new(vec![
        MachineEndpoint::bearer(Method::POST, "/billing/reconcile", TOKEN).unwrap(),
    ])
    .unwrap()
}

fn app() -> Router {
    let app = Router::new()
        .route(
            "/billing/reconcile",
            post(|| async { StatusCode::NO_CONTENT }),
        )
        .route(
            "/billing/reconcile/sibling",
            post(|| async { StatusCode::NO_CONTENT }),
        )
        .route("/browser", post(|| async { StatusCode::NO_CONTENT }));
    apply_security_baseline_with_machine_endpoints(
        app,
        SecurityConfig::default(),
        Environment::Production,
        policy(),
    )
    .unwrap()
}

#[tokio::test]
async fn exact_bearer_route_requires_authentication_and_preserves_baseline() {
    for (path, token, expected) in [
        ("/billing/reconcile", Some(TOKEN), StatusCode::NO_CONTENT),
        ("/billing/reconcile", None, StatusCode::UNAUTHORIZED),
        (
            "/billing/reconcile",
            Some("wrong_credential_0123456789_ABCDEFGHIJKLMNOPQRSTUVWXYZ"),
            StatusCode::UNAUTHORIZED,
        ),
        (
            "/billing/reconcile/sibling",
            Some(TOKEN),
            StatusCode::FORBIDDEN,
        ),
        ("/browser", Some(TOKEN), StatusCode::FORBIDDEN),
    ] {
        let mut request = Request::builder().method("POST").uri(path);
        if let Some(token) = token {
            request = request.header("Authorization", format!("Bearer {token}"));
        }
        let response = app()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), expected, "{path}");
        assert!(response.headers().contains_key("x-content-type-options"));
    }
    for (name, value) in [
        ("Cookie", "session=anything"),
        ("Origin", "https://app.example"),
        ("Sec-Fetch-Site", "same-origin"),
    ] {
        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/billing/reconcile")
                    .header("Authorization", format!("Bearer {TOKEN}"))
                    .header(name, value)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
    let response = app()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/billing/reconcile")
                .header("Authorization", format!("Bearer {TOKEN}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn machine_auth_keeps_waf_body_bounds_and_browser_csrf() {
    let blocked = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/billing/reconcile")
                .header("Authorization", format!("Bearer {TOKEN}"))
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"input":"<script>alert(1)</script>"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(blocked.status(), StatusCode::FORBIDDEN);
    let large = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/billing/reconcile")
                .header("Authorization", format!("Bearer {TOKEN}"))
                .body(Body::from(vec![b'a'; 1024 * 1024 + 1]))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(large.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let browser = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/browser")
                .header("Cookie", "rullst_csrf=matching_token")
                .header("X-CSRF-Token", "matching_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(browser.status(), StatusCode::NO_CONTENT);
}

struct Reject;
#[async_trait::async_trait]
impl MachineRequestVerifier for Reject {
    async fn verify(
        &self,
        _: axum::extract::Request,
    ) -> Result<axum::extract::Request, MachineEndpointError> {
        Err(MachineEndpointError::Unauthorized)
    }
}

#[tokio::test]
async fn typed_webhook_does_not_skip_provider_verification() {
    let policy = MachineEndpointPolicy::new(vec![
        MachineEndpoint::signed_webhook(Method::POST, "/webhook", Reject).unwrap(),
    ])
    .unwrap();
    let app = apply_security_baseline_with_machine_endpoints(
        Router::new().route("/webhook", post(|| async { StatusCode::NO_CONTENT })),
        SecurityConfig::default(),
        Environment::Production,
        policy,
    )
    .unwrap();
    let result = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/webhook")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(result.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn policy_rejects_wildcards_duplicates_and_weak_credentials() {
    for path in [
        "/billing/*",
        "/{id}",
        "/a/../b",
        "/a//b",
        "/a%2fb",
        "relative",
    ] {
        assert!(MachineEndpoint::bearer(Method::POST, path, TOKEN).is_err());
    }
    assert!(MachineEndpoint::bearer(Method::GET, "/machine", TOKEN).is_err());
    assert!(MachineEndpoint::bearer(Method::POST, "/machine", "short").is_err());
    assert!(
        MachineEndpointPolicy::new(vec![
            MachineEndpoint::bearer(Method::POST, "/machine", TOKEN).unwrap(),
            MachineEndpoint::bearer(Method::POST, "/machine", TOKEN).unwrap(),
        ])
        .is_err()
    );
}
