use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
    routing::post,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use rullst_core::{
    config::{Environment, SecurityConfig},
    security::{
        MachineAuthentication, MachineEndpoint, MachineEndpointError, MachineEndpointPolicy,
        MachineRequestVerifier, apply_security_baseline_with_machine_endpoints,
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
    assert!(
        MachineEndpointPolicy::new(
            (0..33)
                .map(
                    |i| MachineEndpoint::bearer(Method::POST, format!("/machine/{i}"), TOKEN)
                        .unwrap()
                )
                .collect()
        )
        .is_err()
    );
    assert_eq!(
        format!("{:?}", policy()),
        "MachineEndpointPolicy { routes: 1 }"
    );
}

// A transport-only extension cannot be forged by an HTTP header.
#[derive(Clone)]
struct FixtureTransportIdentity;

struct TransportVerifier;
#[async_trait::async_trait]
impl MachineRequestVerifier for TransportVerifier {
    async fn verify(
        &self,
        request: axum::extract::Request,
    ) -> Result<axum::extract::Request, MachineEndpointError> {
        if request
            .extensions()
            .get::<FixtureTransportIdentity>()
            .is_none()
        {
            return Err(MachineEndpointError::Unauthorized);
        }
        Ok(request)
    }
}

#[tokio::test]
async fn mutual_tls_requires_transport_identity_and_preserves_the_body() {
    let endpoint =
        MachineEndpoint::mutual_tls(Method::POST, "/machine", TransportVerifier).unwrap();
    assert_eq!(endpoint.authentication(), MachineAuthentication::MutualTls);
    let app = apply_security_baseline_with_machine_endpoints(
        Router::new().route("/machine", post(|body: String| async move { body })),
        SecurityConfig::default(),
        Environment::Production,
        MachineEndpointPolicy::new(vec![endpoint]).unwrap(),
    )
    .unwrap();
    for trusted in [false, true] {
        let mut request = Request::post("/machine")
            .header("X-Client-Cert", "untrusted-proxy-input")
            .body(Body::from("bounded payload"))
            .unwrap();
        if trusted {
            request.extensions_mut().insert(FixtureTransportIdentity);
        }
        let response = app.clone().oneshot(request).await.unwrap();
        assert!(response.headers().contains_key("x-content-type-options"));
        assert_eq!(
            response.status(),
            if trusted {
                StatusCode::OK
            } else {
                StatusCode::UNAUTHORIZED
            }
        );
        if trusted {
            assert_eq!(
                axum::body::to_bytes(response.into_body(), 1024)
                    .await
                    .unwrap(),
                "bounded payload"
            );
        }
    }
}

#[derive(Clone, Copy)]
enum Rewrite {
    None,
    Method,
    Uri,
}

struct SignedFixture {
    key: ring::hmac::Key,
    rewrite: Rewrite,
    calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

#[async_trait::async_trait]
impl MachineRequestVerifier for SignedFixture {
    async fn verify(
        &self,
        request: axum::extract::Request,
    ) -> Result<axum::extract::Request, MachineEndpointError> {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let (mut parts, body) = request.into_parts();
        let bytes = axum::body::to_bytes(body, 1024).await.unwrap();
        let signature = parts
            .headers
            .get("X-Fixture-Signature")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| STANDARD.decode(value).ok())
            .ok_or(MachineEndpointError::Unauthorized)?;
        ring::hmac::verify(&self.key, &bytes, &signature)
            .map_err(|_| MachineEndpointError::Unauthorized)?;
        match self.rewrite {
            Rewrite::None => {}
            Rewrite::Method => parts.method = Method::DELETE,
            Rewrite::Uri => parts.uri = "/webhook?changed=true".parse().unwrap(),
        }
        Ok(Request::from_parts(parts, Body::from(bytes)))
    }
}

#[tokio::test]
async fn webhook_verification_is_idempotent_and_cannot_rewrite_the_request() {
    let key = ring::hmac::Key::generate(ring::hmac::HMAC_SHA256, &ring::rand::SystemRandom::new())
        .unwrap();
    let payload = "event payload";
    let signature = STANDARD.encode(ring::hmac::sign(&key, payload.as_bytes()));
    for rewrite in [Rewrite::None, Rewrite::Method, Rewrite::Uri] {
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let endpoint = MachineEndpoint::signed_webhook(
            Method::POST,
            "/webhook",
            SignedFixture {
                key: key.clone(),
                rewrite,
                calls: calls.clone(),
            },
        )
        .unwrap();
        assert_eq!(
            endpoint.authentication(),
            MachineAuthentication::SignedWebhook
        );
        let policy = MachineEndpointPolicy::new(vec![endpoint]).unwrap();
        let app = apply_security_baseline_with_machine_endpoints(
            Router::new()
                .route("/webhook", post(|body: String| async move { body }))
                .layer(axum::middleware::from_fn(
                    rullst_core::security::csrf_middleware,
                )),
            SecurityConfig::default(),
            Environment::Production,
            policy,
        )
        .unwrap();
        let response = app
            .clone()
            .oneshot(
                Request::post("/webhook")
                    .header("X-Fixture-Signature", &signature)
                    .body(Body::from(payload))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(
            response.status(),
            if matches!(rewrite, Rewrite::None) {
                StatusCode::OK
            } else {
                StatusCode::UNAUTHORIZED
            }
        );
        if matches!(rewrite, Rewrite::None) {
            assert_eq!(
                axum::body::to_bytes(response.into_body(), 1024)
                    .await
                    .unwrap(),
                payload
            );
        }
        let forged = app
            .oneshot(
                Request::post("/webhook")
                    .header("X-Fixture-Signature", &signature)
                    .body(Body::from("altered payload"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(forged.status(), StatusCode::UNAUTHORIZED);
    }
}

#[tokio::test]
async fn development_still_authenticates_machine_routes_and_rejects_duplicate_credentials() {
    let endpoint = MachineEndpoint::bearer(Method::POST, "/machine", TOKEN).unwrap();
    assert_eq!(endpoint.authentication(), MachineAuthentication::Bearer);
    let app = apply_security_baseline_with_machine_endpoints(
        Router::new()
            .route("/machine", post(|| async { StatusCode::NO_CONTENT }))
            .route("/ordinary", post(|| async { StatusCode::NO_CONTENT })),
        SecurityConfig::default(),
        Environment::Development,
        MachineEndpointPolicy::new(vec![endpoint]).unwrap(),
    )
    .unwrap();
    for (path, headers, expected) in [
        ("/machine", 0, StatusCode::UNAUTHORIZED),
        ("/machine", 1, StatusCode::NO_CONTENT),
        ("/machine", 2, StatusCode::UNAUTHORIZED),
        ("/ordinary", 0, StatusCode::NO_CONTENT),
    ] {
        let mut request = Request::post(path);
        for _ in 0..headers {
            request = request.header("Authorization", format!("Bearer {TOKEN}"));
        }
        assert_eq!(
            app.clone()
                .oneshot(request.body(Body::empty()).unwrap())
                .await
                .unwrap()
                .status(),
            expected
        );
    }
}
