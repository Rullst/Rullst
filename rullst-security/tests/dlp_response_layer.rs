//! Response-layer contracts for `DlpResponseLayer`: which responses are
//! rewritten and how their representation metadata stays consistent.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::body::Body;
use axum::http::{HeaderName, HeaderValue, Request, Response, StatusCode, header};
use rullst_security::dlp::DlpResponseLayer;
use std::convert::Infallible;
use tower::{Layer, ServiceExt, service_fn};

const LEAKED_DSN: &str = "postgres://app:hunter2@db/app";

async fn through_dlp(
    status: StatusCode,
    headers: Vec<(HeaderName, &'static str)>,
    body: String,
) -> Response<Body> {
    let service = DlpResponseLayer.layer(service_fn(move |_request: Request<Body>| {
        let mut response = Response::new(Body::from(body.clone()));
        *response.status_mut() = status;
        for (name, value) in &headers {
            response
                .headers_mut()
                .insert(name.clone(), HeaderValue::from_static(value));
        }
        async move { Ok::<_, Infallible>(response) }
    }));
    service
        .oneshot(Request::get("/").body(Body::empty()).unwrap())
        .await
        .unwrap()
}

async fn body_text(response: Response<Body>) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn xml_javascript_and_yaml_responses_are_masked_like_text() {
    for (content_type, body) in [
        ("application/xml", format!("<conn>{LEAKED_DSN}</conn>")),
        (
            "application/soap+xml; charset=utf-8",
            format!("<soap:Body><dsn>{LEAKED_DSN}</dsn></soap:Body>"),
        ),
        (
            "application/atom+xml",
            format!("<entry>{LEAKED_DSN}</entry>"),
        ),
        (
            "application/javascript",
            format!("const DB = \"{LEAKED_DSN}\";"),
        ),
        (
            "Application/X-JavaScript",
            format!("var db='{LEAKED_DSN}';"),
        ),
        ("application/yaml", format!("database_url: {LEAKED_DSN}\n")),
        (
            "application/x-yaml",
            format!("database_url: {LEAKED_DSN}\n"),
        ),
    ] {
        let response = through_dlp(
            StatusCode::OK,
            vec![(header::CONTENT_TYPE, content_type)],
            body,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK, "{content_type}");
        let declared = response
            .headers()
            .get(header::CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<usize>().ok());
        let text = body_text(response).await;
        assert!(!text.contains("hunter2"), "{content_type} was not masked");
        assert!(
            text.contains("postgres://app:*****@db/app"),
            "{content_type}"
        );
        assert_eq!(declared, Some(text.len()), "{content_type}");
    }
}

#[tokio::test]
async fn binary_and_image_responses_still_pass_through_unchanged() {
    for content_type in ["application/octet-stream", "image/png", "application/wasm"] {
        let response = through_dlp(
            StatusCode::OK,
            vec![(header::CONTENT_TYPE, content_type)],
            LEAKED_DSN.to_string(),
        )
        .await;
        let unchanged = body_text(response).await == LEAKED_DSN;
        assert!(unchanged, "{content_type} must bypass DLP unchanged");
    }
}

#[tokio::test]
async fn masked_partial_content_is_withheld_instead_of_losing_content_range() {
    let response = through_dlp(
        StatusCode::PARTIAL_CONTENT,
        vec![
            (header::CONTENT_TYPE, "text/plain; charset=utf-8"),
            (header::CONTENT_RANGE, "bytes 0-36/5000"),
            (header::ACCEPT_RANGES, "bytes"),
        ],
        "log line with AKIAIOSFODNN7EXAMPLE".to_string(),
    )
    .await;

    // Never a 206 without Content-Range, and never the unmasked range.
    assert_ne!(response.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    assert!(response.headers().get(header::CONTENT_RANGE).is_none());
    assert_eq!(
        response.headers().get(header::CACHE_CONTROL),
        Some(&HeaderValue::from_static("no-store"))
    );
    let text = body_text(response).await;
    assert!(!text.contains("AKIAIOSFODNN7EXAMPLE"));
}

#[tokio::test]
async fn clean_partial_content_keeps_its_range_metadata() {
    let response = through_dlp(
        StatusCode::PARTIAL_CONTENT,
        vec![
            (header::CONTENT_TYPE, "text/plain; charset=utf-8"),
            (header::CONTENT_RANGE, "bytes 0-21/5000"),
        ],
        "an ordinary log line..".to_string(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        response.headers().get(header::CONTENT_RANGE),
        Some(&HeaderValue::from_static("bytes 0-21/5000"))
    );
    assert_eq!(body_text(response).await, "an ordinary log line..");
}
