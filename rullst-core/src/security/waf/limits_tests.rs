//! Which request bodies the WAF inspects, and its one-mebibyte limit.

use super::*;
use axum::{Router, body::Bytes, http::Request, routing::post};
use tower::ServiceExt;

fn echo_app() -> Router {
    Router::new()
        .route("/items", post(|body: Bytes| async move { body }))
        .route_layer(axum::middleware::from_fn(waf_middleware))
}

async fn status(content_type: &str, length: Option<&str>, body: impl Into<Body>) -> StatusCode {
    let mut request = Request::post("/items").header(header::CONTENT_TYPE, content_type);
    if let Some(length) = length {
        request = request.header(header::CONTENT_LENGTH, length);
    }
    let request = request.body(body.into()).unwrap();
    echo_app().oneshot(request).await.unwrap().status()
}

#[tokio::test]
async fn bodies_up_to_one_mebibyte_are_inspected() {
    assert_eq!(
        status("text/plain", Some("1048576"), vec![b'a'; 1_048_576]).await,
        StatusCode::OK
    );
    assert_eq!(
        status("text/plain", Some("5"), "hello").await,
        StatusCode::OK
    );
    // A larger declared length is refused before the body is read.
    assert_eq!(
        status("text/plain", Some("1048577"), "hello").await,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert_eq!(
        status("text/plain", None, vec![b'a'; 1_048_577]).await,
        StatusCode::PAYLOAD_TOO_LARGE
    );
}

#[tokio::test]
async fn only_declared_textual_bodies_are_inspected_and_only_json_by_value() {
    assert_eq!(
        status("application/octet-stream", None, "../../etc/passwd").await,
        StatusCode::OK
    );
    // Without a JSON media type, JSON-looking text is inspected as a whole.
    let json_text = r#"{"op":"or","v":"a=b","args":"--force"}"#;
    assert_eq!(
        status("text/plain", None, json_text).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        status("application/json", None, json_text).await,
        StatusCode::OK
    );
}
