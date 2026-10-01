//! Guards of the hot-reload HMR script injection.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use axum::body::{Body, to_bytes};
use axum::http::{HeaderValue, Method, Request, StatusCode, header};
use axum::response::Response;
use tower::ServiceExt;

fn html_response(status: StatusCode, body: Body) -> Response {
    let mut response = Response::new(body);
    *response.status_mut() = status;
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    headers.insert(header::ETAG, HeaderValue::from_static("\"v1\""));
    headers.insert(
        header::LAST_MODIFIED,
        HeaderValue::from_static("Wed, 01 Jan 2025 00:00:00 GMT"),
    );
    response
}

fn router() -> axum::Router {
    axum::Router::new()
        .route(
            "/page",
            axum::routing::get(|| async {
                html_response(StatusCode::OK, "<body>page</body>".into())
            }),
        )
        .route(
            "/gzip",
            axum::routing::get(|| async {
                let mut response =
                    html_response(StatusCode::OK, Body::from(vec![0x1f, 0x8b, 0x08, 0x00]));
                response
                    .headers_mut()
                    .insert(header::CONTENT_ENCODING, HeaderValue::from_static("gzip"));
                response
            }),
        )
        .route(
            "/latin1",
            axum::routing::get(|| async {
                html_response(StatusCode::OK, Body::from(b"<body>Jos\xe9</body>".to_vec()))
            }),
        )
        .route(
            "/partial",
            axum::routing::get(|| async {
                html_response(StatusCode::PARTIAL_CONTENT, "<body>par".into())
            }),
        )
        .layer(axum::middleware::from_fn(inject_hmr_script))
}

async fn send(request: Request<Body>) -> (Response<()>, Vec<u8>) {
    let response = router().oneshot(request).await.unwrap();
    let (parts, body) = response.into_parts();
    let bytes = to_bytes(body, 64 * 1024).await.unwrap().to_vec();
    (Response::from_parts(parts, ()), bytes)
}

#[tokio::test]
async fn full_documents_get_one_nonced_script_and_lose_validators() {
    let mut request = Request::get("/page").body(Body::empty()).unwrap();
    let nonce = crate::security::CspNonce::get_or_insert(request.extensions_mut());
    let (response, body) = send(request).await;
    let body = String::from_utf8(body).unwrap();

    assert!(body.contains(&format!(
        "<script src=\"{HMR_CLIENT_PATH}\" nonce=\"{}\" defer></script>",
        nonce.as_str()
    )));
    assert!(body.ends_with("</body>"));
    assert!(!response.headers().contains_key(header::ETAG));
    assert!(!response.headers().contains_key(header::LAST_MODIFIED));
    assert!(HMR_CLIENT.contains("window.__rullstHmr"));
}

#[tokio::test]
async fn fragments_head_partial_encoded_and_non_utf8_bodies_are_untouched() {
    let (response, body) = send(
        Request::get("/page")
            .header("HX-Request", "true")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(body, b"<body>page</body>");
    assert!(response.headers().contains_key(header::ETAG));

    let (_, body) = send(
        Request::builder()
            .method(Method::HEAD)
            .uri("/page")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert!(!String::from_utf8_lossy(&body).contains(HMR_CLIENT_PATH));

    let (response, body) = send(Request::get("/partial").body(Body::empty()).unwrap()).await;
    assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(body, b"<body>par");

    let (response, body) = send(Request::get("/gzip").body(Body::empty()).unwrap()).await;
    assert_eq!(response.headers()[header::CONTENT_ENCODING], "gzip");
    assert_eq!(body, [0x1f, 0x8b, 0x08, 0x00]);

    let (_, body) = send(Request::get("/latin1").body(Body::empty()).unwrap()).await;
    assert_eq!(body, b"<body>Jos\xe9</body>");
}
