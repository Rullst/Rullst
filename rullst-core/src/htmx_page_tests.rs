//! Tests for the htmx requests that receive a whole page and for
//! `Vary: HX-Request`.

use super::*;
use axum::http::Request;

async fn htmx_request(headers: &[(&str, &str)]) -> HtmxRequest {
    let mut request = Request::builder();
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let (mut parts, ()) = request.body(()).unwrap().into_parts();
    HtmxRequest::from_request_parts(&mut parts, &())
        .await
        .unwrap()
}

#[tokio::test]
async fn history_restores_and_boosted_requests_receive_the_whole_page() {
    let content = || crate::html! { <p>"fragment"</p> };
    // htmx 1.9.12 and htmx 2 (its default `historyRestoreAsHxRequest`) send
    // both headers when the history cache misses and replace the whole
    // <body> with the response.
    let restore = htmx_request(&[
        ("HX-Request", "true"),
        ("HX-History-Restore-Request", "true"),
    ])
    .await;
    assert!(restore.is_htmx && restore.is_history_restore() && !restore.is_boosted());
    let boosted = htmx_request(&[("HX-Request", "true"), ("HX-Boosted", "true")]).await;
    assert!(boosted.is_htmx && boosted.is_boosted() && !boosted.is_history_restore());
    for request in [&restore, &boosted] {
        for page in [
            render_page(request, "T", content()).0,
            render_page_with_lang(request, "en", "T", content()).0,
            render_page_with_head(request, "en", "T", Markup::default(), content()).0,
        ] {
            assert!(page.starts_with("<!DOCTYPE html>"), "{page}");
            assert!(
                page.contains("<body class=\"h-full\"><p>fragment</p></body>"),
                "{page}"
            );
        }
    }

    // A targeted swap still receives the fragment, also with a header that is
    // not `true`.
    for headers in [
        &[("HX-Request", "true")][..],
        &[
            ("HX-Request", "true"),
            ("HX-History-Restore-Request", "false"),
        ],
        &[("HX-Request", "true"), ("HX-Boosted", "1")],
    ] {
        let request = htmx_request(headers).await;
        assert!(!request.is_history_restore() && !request.is_boosted());
        assert_eq!(render_page(&request, "T", content()).0, "<p>fragment</p>");
    }
    // Without `HX-Request` the page is the one a browser navigation receives.
    let plain = htmx_request(&[("HX-History-Restore-Request", "true")]).await;
    assert!(
        render_page(&plain, "T", content())
            .0
            .starts_with("<!DOCTYPE html>")
    );
}

#[test]
fn html_responses_vary_on_hx_request_once() {
    let vary = |content_type: Option<&'static str>, existing: &[&'static str]| {
        let mut headers = axum::http::HeaderMap::new();
        if let Some(content_type) = content_type {
            headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
        }
        for value in existing {
            headers.append(header::VARY, HeaderValue::from_static(value));
        }
        vary_html_on_htmx_request(&mut headers);
        headers
            .get_all(header::VARY)
            .iter()
            .map(|value| value.to_str().unwrap().to_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(vary(Some("text/html; charset=utf-8"), &[]), ["HX-Request"]);
    assert_eq!(vary(Some("Text/HTML"), &[]), ["HX-Request"]);
    assert_eq!(
        vary(Some("text/html"), &["Accept-Encoding"]),
        ["Accept-Encoding", "HX-Request"]
    );
    for kept in ["accept-encoding, hx-request", "HX-Request", "*"] {
        assert_eq!(vary(Some("text/html"), &[kept]), [kept]);
    }
    for other in [
        None,
        Some("application/json"),
        Some("text/plain"),
        Some("text/html-sandboxed"),
    ] {
        assert!(vary(other, &[]).is_empty(), "{other:?}");
    }
}

#[tokio::test]
async fn the_middleware_marks_pages_and_fragments_of_a_route() {
    use tower::ServiceExt;
    async fn page(htmx: HtmxRequest) -> Html<String> {
        render_page(&htmx, "T", crate::html! { <p>"x"</p> })
    }
    let app = axum::Router::new()
        .route("/", axum::routing::get(page))
        .route("/api", axum::routing::get(|| async { "plain" }))
        .layer(axum::middleware::from_fn(vary_on_htmx_request));
    for request in [
        Request::get("/").body(axum::body::Body::empty()).unwrap(),
        Request::get("/")
            .header("HX-Request", "true")
            .body(axum::body::Body::empty())
            .unwrap(),
    ] {
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.headers().get(header::VARY).unwrap(), "HX-Request");
    }
    let response = app
        .oneshot(
            Request::get("/api")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(response.headers().get(header::VARY).is_none());
}
