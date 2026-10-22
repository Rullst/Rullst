//! Admission of monitor requests: credentials, transport, origin and limits
//! (`TM-STUDIO-2`, `MONITOR-02`, `-03`, `-06`, `-07`, `-08`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::test_support::{BASE64_TOKEN, HEX_TOKEN, ROTATED_TOKEN, bearer, get, read};
use super::{
    Installed, MONITOR_PATH, Monitor, MonitorAccess, MonitorAuthorizer, MonitorAuthorizerError,
    MonitorVerifiedTls,
};
use crate::config::Environment;
use crate::security::{TrustedProxyConfig, TrustedProxyLayer};
use axum::Router;
use axum::http::request::Parts;
use axum::http::{Method, StatusCode, header};
use tower::ServiceExt;

const CLIENT: &str = "192.0.2.10:40000";

fn app(monitor: Monitor, environment: Environment) -> Router {
    let installed = Installed::prepare(monitor, environment)
        .expect("valid monitor")
        .expect("credentials configured");
    let router = Router::new().route("/", axum::routing::get(|| async { "home" }));
    installed.record(installed.mount(router))
}

fn token_app(environment: Environment) -> Router {
    app(
        Monitor::new()
            .with_token(BASE64_TOKEN)
            .with_token(ROTATED_TOKEN),
        environment,
    )
}

async fn status_of(app: &Router, peer: &str, headers: &[(&str, &str)]) -> StatusCode {
    app.clone()
        .oneshot(get(MONITOR_PATH, peer, headers))
        .await
        .unwrap()
        .status()
}

#[tokio::test]
async fn missing_and_invalid_credentials_get_the_same_401() {
    let app = token_app(Environment::Development);
    let wrong = bearer(HEX_TOKEN);
    let cases: [&[(&str, &str)]; 5] = [
        &[],
        &[(wrong.0, wrong.1.as_str())],
        &[("authorization", "Bearer")],
        &[("authorization", "Basic YWRtaW46YWRtaW4=")],
        &[(
            "authorization",
            "Bearer DjxqcYY6VXbaPLLGGkjMN3aJCjqkMo8TcEjFfXD6f7",
        )],
    ];
    for (index, headers) in cases.iter().enumerate() {
        let peer = format!("192.0.2.{}:40000", index + 20);
        let response = app
            .clone()
            .oneshot(get(MONITOR_PATH, &peer, headers))
            .await
            .unwrap();
        let headers = response.headers().clone();
        let (status, body) = read(response).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "case {index}");
        assert!(body.is_empty(), "case {index}");
        assert_eq!(
            headers[header::WWW_AUTHENTICATE],
            "Bearer realm=\"rullst-monitor\""
        );
        assert_eq!(headers[header::CACHE_CONTROL], "no-store");
        assert_eq!(headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
    }
}

#[tokio::test]
async fn a_configured_or_rotated_token_reads_the_document() {
    let app = token_app(Environment::Development);
    for token in [BASE64_TOKEN, ROTATED_TOKEN] {
        let authorization = bearer(token);
        let response = app
            .clone()
            .oneshot(get(
                MONITOR_PATH,
                CLIENT,
                &[(authorization.0, authorization.1.as_str())],
            ))
            .await
            .unwrap();
        let headers = response.headers().clone();
        let (status, body) = read(response).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(headers[header::CONTENT_TYPE], "application/json");
        assert_eq!(headers[header::CACHE_CONTROL], "no-store");
        assert_eq!(headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
        assert!(headers.get(header::ACCESS_CONTROL_ALLOW_ORIGIN).is_none());
        let document: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(document["schema"], "rullst.monitor.v1");
    }
}

#[tokio::test]
async fn other_methods_are_not_allowed_and_not_cached() {
    let app = token_app(Environment::Development);
    let mut request = get(MONITOR_PATH, CLIENT, &[]);
    *request.method_mut() = Method::POST;
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
}

#[tokio::test]
async fn production_refuses_plain_http_before_evaluating_credentials() {
    let authorization = bearer(BASE64_TOKEN);
    let headers = [(authorization.0, authorization.1.as_str())];
    for environment in [Environment::Production, Environment::Staging] {
        let response = token_app(environment)
            .oneshot(get(MONITOR_PATH, CLIENT, &headers))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UPGRADE_REQUIRED);
        assert_eq!(response.headers()[header::UPGRADE], "TLS/1.2, HTTP/1.1");
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    }
    // An absolute-form `https://` target is chosen by the client, not TLS.
    let forged = get("https://shop.example/_rullst/monitor/v1", CLIENT, &headers);
    let response = token_app(Environment::Production)
        .oneshot(forged)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UPGRADE_REQUIRED);

    // The application's own TLS listener marks verified requests.
    let mut verified = get(MONITOR_PATH, CLIENT, &headers);
    verified
        .extensions_mut()
        .insert(MonitorVerifiedTls::from_trusted_tls_termination());
    let response = token_app(Environment::Production)
        .oneshot(verified)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    for environment in [Environment::Development, Environment::Test] {
        assert_eq!(
            status_of(&token_app(environment), CLIENT, &headers).await,
            StatusCode::OK
        );
    }
}

#[tokio::test]
async fn only_a_trusted_proxy_reports_https() {
    let proxies = TrustedProxyConfig::new(["10.0.0.0/8"])
        .unwrap()
        .trust_forwarded_proto(true);
    let app = token_app(Environment::Production).layer(TrustedProxyLayer::new(proxies));
    let authorization = bearer(BASE64_TOKEN);
    let headers = [
        (authorization.0, authorization.1.as_str()),
        ("x-forwarded-proto", "https"),
        ("x-forwarded-for", "198.51.100.7"),
    ];
    assert_eq!(
        status_of(&app, "203.0.113.5:40000", &headers).await,
        StatusCode::UPGRADE_REQUIRED
    );
    assert_eq!(
        status_of(&app, "10.0.0.2:40000", &headers).await,
        StatusCode::OK
    );
    let plain = [
        (authorization.0, authorization.1.as_str()),
        ("x-forwarded-proto", "http"),
    ];
    assert_eq!(
        status_of(&app, "10.0.0.2:40000", &plain).await,
        StatusCode::UPGRADE_REQUIRED
    );
}

#[tokio::test]
async fn loopback_plaintext_is_an_explicit_opt_in_for_direct_local_clients() {
    let authorization = bearer(BASE64_TOKEN);
    let local = [
        (authorization.0, authorization.1.as_str()),
        ("host", "localhost:3000"),
    ];
    let tunnel = app(
        Monitor::new()
            .with_token(BASE64_TOKEN)
            .allow_loopback_plaintext(true),
        Environment::Production,
    );
    assert_eq!(
        status_of(&tunnel, "127.0.0.1:50000", &local).await,
        StatusCode::OK
    );
    assert_eq!(
        status_of(&tunnel, "[::1]:50000", &local).await,
        StatusCode::OK
    );
    // A same-host reverse proxy adds forwarding headers.
    let proxied = [
        (authorization.0, authorization.1.as_str()),
        ("host", "localhost:3000"),
        ("x-forwarded-for", "198.51.100.7"),
    ];
    assert_eq!(
        status_of(&tunnel, "127.0.0.1:50000", &proxied).await,
        StatusCode::UPGRADE_REQUIRED
    );
    let public_host = [
        (authorization.0, authorization.1.as_str()),
        ("host", "shop.example"),
    ];
    assert_eq!(
        status_of(&tunnel, "127.0.0.1:50000", &public_host).await,
        StatusCode::UPGRADE_REQUIRED
    );
    assert_eq!(
        status_of(&tunnel, CLIENT, &local).await,
        StatusCode::UPGRADE_REQUIRED
    );
    // A client resolved through a trusted loopback proxy is not local.
    let behind_proxy = app(
        Monitor::new()
            .with_token(BASE64_TOKEN)
            .allow_loopback_plaintext(true),
        Environment::Production,
    )
    .layer(TrustedProxyLayer::new(
        TrustedProxyConfig::new(["127.0.0.1/32"]).unwrap(),
    ));
    assert_eq!(
        status_of(&behind_proxy, "127.0.0.1:50000", &proxied).await,
        StatusCode::UPGRADE_REQUIRED
    );
    assert_eq!(
        status_of(&behind_proxy, "127.0.0.1:50000", &local).await,
        StatusCode::OK
    );
    // Without the opt-in a loopback client still needs TLS.
    assert_eq!(
        status_of(
            &token_app(Environment::Production),
            "127.0.0.1:50000",
            &local
        )
        .await,
        StatusCode::UPGRADE_REQUIRED
    );
}

#[tokio::test]
async fn browser_cross_origin_requests_are_refused_even_with_a_token() {
    let app = token_app(Environment::Development);
    let authorization = bearer(BASE64_TOKEN);
    let with = |name: &'static str, value: &'static str| {
        [
            (authorization.0, authorization.1.clone()),
            (name, value.to_string()),
        ]
    };
    for (name, value, expected) in [
        ("origin", "https://attacker.example", StatusCode::FORBIDDEN),
        ("origin", "null", StatusCode::FORBIDDEN),
        ("sec-fetch-site", "cross-site", StatusCode::FORBIDDEN),
        ("sec-fetch-site", "Same-Site", StatusCode::FORBIDDEN),
        ("sec-fetch-site", "same-origin", StatusCode::OK),
        ("sec-fetch-site", "none", StatusCode::OK),
    ] {
        let headers = with(name, value);
        let headers: Vec<(&str, &str)> = headers.iter().map(|(n, v)| (*n, v.as_str())).collect();
        let response = app
            .clone()
            .oneshot(get(MONITOR_PATH, CLIENT, &headers))
            .await
            .unwrap();
        let cors = response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .cloned();
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        let (status, body) = read(response).await;
        assert_eq!(status, expected, "{name}: {value}");
        assert!(cors.is_none());
        if expected == StatusCode::FORBIDDEN {
            assert!(body.is_empty());
        }
    }
}

struct RoleHeader;

impl MonitorAuthorizer for RoleHeader {
    async fn authorize(&self, request: &Parts) -> Result<MonitorAccess, MonitorAuthorizerError> {
        match request.headers.get("x-role").map(|role| role.as_bytes()) {
            Some(b"ops") => Ok(MonitorAccess::Allow),
            Some(b"user") => Ok(MonitorAccess::Deny),
            Some(b"broken") => Err(MonitorAuthorizerError::new("session store unavailable")),
            _ => Ok(MonitorAccess::SignIn),
        }
    }
}

#[tokio::test]
async fn the_application_authorizer_decides_without_a_valid_token() {
    let app = app(
        Monitor::new().with_authorizer(RoleHeader),
        Environment::Development,
    );
    for (role, expected) in [
        (Some("ops"), StatusCode::OK),
        (Some("user"), StatusCode::FORBIDDEN),
        (Some("broken"), StatusCode::SERVICE_UNAVAILABLE),
        (None, StatusCode::UNAUTHORIZED),
    ] {
        let headers: Vec<(&str, &str)> = role.map(|role| ("x-role", role)).into_iter().collect();
        assert_eq!(
            status_of(&app, CLIENT, &headers).await,
            expected,
            "{role:?}"
        );
    }
    // Only the authorizer is configured: a bearer value is not a token.
    let authorization = bearer(BASE64_TOKEN);
    assert_eq!(
        status_of(&app, CLIENT, &[(authorization.0, authorization.1.as_str())]).await,
        StatusCode::UNAUTHORIZED
    );

    let both = app_with_token_and_authorizer();
    assert_eq!(
        status_of(
            &both,
            CLIENT,
            &[
                (authorization.0, authorization.1.as_str()),
                ("x-role", "user")
            ]
        )
        .await,
        StatusCode::OK
    );
    let wrong = bearer(HEX_TOKEN);
    assert_eq!(
        status_of(
            &both,
            CLIENT,
            &[(wrong.0, wrong.1.as_str()), ("x-role", "ops")]
        )
        .await,
        StatusCode::OK
    );
}

fn app_with_token_and_authorizer() -> Router {
    app(
        Monitor::new()
            .with_token(BASE64_TOKEN)
            .with_authorizer(RoleHeader),
        Environment::Development,
    )
}

#[tokio::test]
async fn each_client_has_its_own_request_budget() {
    let app = token_app(Environment::Development);
    let authorization = bearer(BASE64_TOKEN);
    let headers = [(authorization.0, authorization.1.as_str())];
    // Requests without a credential are cheap and still spend the budget.
    for _ in 0..30 {
        assert_eq!(status_of(&app, CLIENT, &[]).await, StatusCode::UNAUTHORIZED);
    }
    let response = app
        .clone()
        .oneshot(get(MONITOR_PATH, CLIENT, &headers))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(response.headers()[header::RETRY_AFTER], "1");
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    assert_eq!(
        status_of(&app, "192.0.2.11:40000", &headers).await,
        StatusCode::OK
    );
}

#[tokio::test]
async fn presented_invalid_tokens_lock_the_client_out_without_evaluation() {
    let app = token_app(Environment::Development);
    let wrong = bearer(HEX_TOKEN);
    let valid = bearer(BASE64_TOKEN);
    // Requests without a credential never count as failures.
    for _ in 0..12 {
        assert_eq!(status_of(&app, CLIENT, &[]).await, StatusCode::UNAUTHORIZED);
    }
    for attempt in 0..10 {
        assert_eq!(
            status_of(&app, CLIENT, &[(wrong.0, wrong.1.as_str())]).await,
            StatusCode::UNAUTHORIZED,
            "attempt {attempt}"
        );
    }
    let response = app
        .clone()
        .oneshot(get(MONITOR_PATH, CLIENT, &[(valid.0, valid.1.as_str())]))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(response.headers()[header::RETRY_AFTER], "90");
    assert_eq!(
        status_of(&app, "192.0.2.12:40000", &[(valid.0, valid.1.as_str())]).await,
        StatusCode::OK
    );
}

#[test]
fn without_a_credential_nothing_is_mounted() {
    assert!(
        Installed::prepare(Monitor::new(), Environment::Production)
            .unwrap()
            .is_none()
    );
    let mut monitor = Monitor::new();
    monitor.add_environment_tokens(Some(" , ,"));
    assert!(
        Installed::prepare(monitor, Environment::Production)
            .unwrap()
            .is_none()
    );
    monitor_from_env_is_mounted();
}

fn monitor_from_env_is_mounted() {
    let mut monitor = Monitor::new();
    monitor.add_environment_tokens(Some(&format!(" {BASE64_TOKEN} ,{ROTATED_TOKEN},")));
    assert_eq!(format!("{monitor:?}").matches("tokens: 2").count(), 1);
    assert!(!format!("{monitor:?}").contains(BASE64_TOKEN));
    assert!(
        Installed::prepare(monitor, Environment::Production)
            .unwrap()
            .is_some()
    );
}
