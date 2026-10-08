//! `GET /_rullst/errors/{id}` is mounted only in debug Development builds
//! and answers only direct loopback requests.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::config::{Environment, SecurityConfig};
use crate::server::builder::Server;
use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode};
use std::net::SocketAddr;
use tower::ServiceExt;

async fn panics() {
    panic!("dev errors probe panic");
}

fn app(environment: Environment) -> axum::Router {
    Server::new(crate::Router::new().route("/probe", axum::routing::get(panics)))
        .into_static_app(SecurityConfig::default(), environment)
        .unwrap()
}

async fn send(
    app: &axum::Router,
    path: &str,
    peer: &str,
    headers: &[(&str, &str)],
) -> (StatusCode, String) {
    let mut builder = Request::builder()
        .uri(path)
        .header("host", "127.0.0.1:3000");
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let mut request = builder
        .body(Body::from("password=body-secret-value"))
        .unwrap();
    request
        .extensions_mut()
        .insert(ConnectInfo(peer.parse::<SocketAddr>().unwrap()));
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, String::from_utf8_lossy(&body).into_owned())
}

#[cfg(debug_assertions)]
fn fix_id(page: &str) -> String {
    page.split("cargo rullst ai fix ")
        .nth(1)
        .map(|rest| rest.chars().take_while(char::is_ascii_hexdigit).collect())
        .expect("fix command on the error page")
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn error_context_is_served_only_to_direct_loopback_requests() {
    let development = app(Environment::Development);
    let (status, page) = send(
        &development,
        "/probe?token=query-secret-value",
        "127.0.0.1:40100",
        &[
            ("authorization", "Bearer header-secret-value"),
            ("cookie", "session=cookie-secret-value"),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    let id = fix_id(&page);
    let path = format!("/_rullst/errors/{id}");

    let (status, body) = send(&development, &path, "127.0.0.1:40101", &[]).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["schema"], "rullst.error-context.v1");
    assert_eq!(json["id"], id.as_str());
    assert_eq!(json["message"], "dev errors probe panic");
    assert_eq!(json["method"], "GET");
    assert_eq!(json["path"], "/probe");
    for secret in [
        "query-secret",
        "header-secret",
        "cookie-secret",
        "body-secret",
    ] {
        assert!(!body.contains(secret), "{secret} leaked into the context");
    }

    // Other machines, rebinding pages, proxies and unknown ids get 404.
    for (peer, headers) in [
        ("192.0.2.30:40102", vec![]),
        ("[::ffff:10.0.0.1]:40103", vec![]),
        ("127.0.0.1:40104", vec![("x-forwarded-for", "192.0.2.30")]),
        ("127.0.0.1:40105", vec![("origin", "http://evil.example")]),
    ] {
        let (status, body) = send(&development, &path, peer, &headers).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{peer} {headers:?}");
        assert!(!body.contains("dev errors probe panic"));
    }
    let mut rebinding = Request::builder()
        .uri(&path)
        .header("host", "evil.example:3000")
        .body(Body::empty())
        .unwrap();
    rebinding.extensions_mut().insert(ConnectInfo(
        "127.0.0.1:40106".parse::<SocketAddr>().unwrap(),
    ));
    let response = development.clone().oneshot(rebinding).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    for unknown in [
        "/_rullst/errors/0123456789abcdef0123456789abcdef",
        "/_rullst/errors/not-an-id",
    ] {
        let (status, _) = send(&development, unknown, "127.0.0.1:40107", &[]).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{unknown}");
    }
}

#[tokio::test]
async fn error_context_route_is_absent_outside_debug_development() {
    let id = crate::error_console::store::record("outside", None, None, "GET", "/x");
    let path = format!("/_rullst/errors/{id}");
    let mut environments = vec![
        Environment::Staging,
        Environment::Production,
        Environment::Test,
    ];
    if !cfg!(debug_assertions) {
        environments.push(Environment::Development);
    }
    for environment in environments {
        let (status, body) = send(&app(environment), &path, "127.0.0.1:40110", &[]).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{environment:?}");
        assert!(!body.contains("outside"), "{environment:?}");
    }
}
