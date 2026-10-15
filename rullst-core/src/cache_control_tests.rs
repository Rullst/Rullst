#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::config::{Environment, SecurityConfig};
use axum::body::Body;
use axum::response::IntoResponse;
use axum::routing::get;
use tower::ServiceExt;

#[test]
fn policies_render_whole_seconds() {
    assert_eq!(
        CacheControl::public(Duration::from_secs(300)).header_value(),
        "public, max-age=300"
    );
    assert_eq!(
        CacheControl::private(Duration::from_millis(1_999)).header_value(),
        "private, max-age=1"
    );
    assert_eq!(
        CacheControl::public(Duration::ZERO).header_value(),
        "public, max-age=0"
    );
}

#[test]
fn a_handler_can_return_a_policy_with_its_response() {
    let response = (CacheControl::private(Duration::from_secs(60)), "body").into_response();
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "private, max-age=60"
    );
}

#[test]
fn only_successful_and_not_modified_responses_without_a_policy_are_labelled() {
    let value = HeaderValue::from_static("public, max-age=5");
    for (status, labelled) in [
        (StatusCode::OK, true),
        (StatusCode::PARTIAL_CONTENT, true),
        (StatusCode::NOT_MODIFIED, true),
        (StatusCode::FOUND, false),
        (StatusCode::NOT_FOUND, false),
        (StatusCode::INTERNAL_SERVER_ERROR, false),
    ] {
        let mut headers = HeaderMap::new();
        label(status, &mut headers, value.clone());
        assert_eq!(
            headers.contains_key(header::CACHE_CONTROL),
            labelled,
            "{status}"
        );
    }
    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    label(StatusCode::OK, &mut headers, value);
    assert_eq!(headers[header::CACHE_CONTROL], "no-cache");
}

#[test]
fn shared_caching_needs_public_or_s_maxage_without_private_or_no_store() {
    for (values, shared) in [
        (&["public, max-age=60"][..], true),
        (&["max-age=60, S-MAXAGE=600"], true),
        (&["max-age=60", "Public"], true),
        (&["private, max-age=60"], false),
        (&["public, no-store"], false),
        (&["public", "private"], false),
        (&["max-age=60"], false),
        (&["no-cache"], false),
        (&["publicity"], false),
        (&[], false),
    ] {
        let mut headers = HeaderMap::new();
        for value in values {
            headers.append(header::CACHE_CONTROL, HeaderValue::from_static(value));
        }
        assert_eq!(allows_shared_caching(&headers), shared, "{values:?}");
    }
}

fn app(environment: Environment) -> axum::Router {
    let router = axum::Router::new()
        .route(
            "/calendar.ics",
            get(|| async { "BEGIN:VCALENDAR" }).layer(public(Duration::from_secs(300))),
        )
        .route(
            "/report",
            get(|| async { "report" }).layer(private(Duration::from_secs(3600))),
        )
        .route(
            "/missing",
            get(|| async { StatusCode::NOT_FOUND }).layer(public(Duration::from_secs(300))),
        )
        .route(
            "/own",
            get(|| async { ([(header::CACHE_CONTROL, "no-cache")], "own") })
                .layer(public(Duration::from_secs(300))),
        );
    crate::security::apply_security_baseline(router, SecurityConfig::default(), environment)
        .unwrap()
}

async fn head(app: &axum::Router, path: &str) -> (StatusCode, HeaderMap) {
    let response = app
        .clone()
        .oneshot(Request::get(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    (response.status(), response.headers().clone())
}

#[tokio::test]
async fn behind_the_production_baseline_policies_replace_no_store_only_where_set() {
    let app = app(Environment::Production);
    let (status, headers) = head(&app, "/calendar.ics").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CACHE_CONTROL], "public, max-age=300");
    // A publicly cacheable response never sets the per-visitor CSRF cookie.
    assert!(!headers.contains_key(header::SET_COOKIE));

    let (_, headers) = head(&app, "/report").await;
    assert_eq!(headers[header::CACHE_CONTROL], "private, max-age=3600");
    let cookie = headers[header::SET_COOKIE].to_str().unwrap();
    assert!(cookie.starts_with("rullst_csrf="), "{cookie}");

    let (status, headers) = head(&app, "/missing").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(headers[header::CACHE_CONTROL], "no-store");

    let (_, headers) = head(&app, "/own").await;
    assert_eq!(headers[header::CACHE_CONTROL], "no-cache");
}

#[tokio::test]
async fn the_layer_works_without_the_baseline_too() {
    let app = app(Environment::Development);
    let (_, headers) = head(&app, "/calendar.ics").await;
    assert_eq!(headers[header::CACHE_CONTROL], "public, max-age=300");
    let (_, headers) = head(&app, "/missing").await;
    assert!(!headers.contains_key(header::CACHE_CONTROL));
}
