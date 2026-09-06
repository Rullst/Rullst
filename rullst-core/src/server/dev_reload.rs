//! Same-origin generation polling for the supervised development process.
//! A generation is an opaque readiness marker, never an authentication credential.
use axum::{
    Router,
    body::Body,
    extract::{Request, State},
    http::{HeaderValue, header},
    middleware::{self, Next},
    response::Response,
    routing::get,
};
use std::sync::Arc;

const PATH: &str = "/_rullst/dev-reload.js";
const LIMIT: usize = 10 * 1024 * 1024;
const SCRIPT: &str = r#"(() => {
  'use strict';
  const generation = document.currentScript?.dataset.generation;
  if (!/^[a-f0-9]{32}$/.test(generation || '') || window.__rullstDevReload) return;
  window.__rullstDevReload = true;
  let delay = 500;
  const poll = async () => {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 2000);
    try {
      const response = await fetch('/_rullst/dev-generation', {
        cache: 'no-store', credentials: 'same-origin', signal: controller.signal
      });
      const next = response.headers.get('x-rullst-dev-generation');
      if (response.ok && /^[a-f0-9]{32}$/.test(next || '') && next !== generation) {
        window.location.reload();
        return;
      }
      delay = 500;
    } catch (_) { delay = Math.min(delay * 2, 5000); }
    finally { clearTimeout(timeout); }
    setTimeout(poll, delay);
  };
  setTimeout(poll, delay);
})();
"#;

pub(super) fn mount(router: Router, development: bool, generation: Option<String>) -> Router {
    if !cfg!(debug_assertions) || !development {
        return router;
    }
    let Some(generation) = generation.filter(|value| {
        value.len() == 32
            && value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    }) else {
        return router;
    };
    let generation: Arc<str> = Arc::from(generation);
    let probe = generation.clone();
    router
        .route(
            "/_rullst/dev-generation",
            get(move || {
                let probe = probe.clone();
                async move {
                    let mut response = Response::new(Body::empty());
                    if let Ok(value) = HeaderValue::from_str(&probe) {
                        response
                            .headers_mut()
                            .insert("x-rullst-dev-generation", value);
                    }
                    response
                        .headers_mut()
                        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
                    response
                }
            }),
        )
        .route(
            PATH,
            get(|| async {
                let mut response = Response::new(Body::from(SCRIPT));
                response.headers_mut().insert(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/javascript; charset=utf-8"),
                );
                response
                    .headers_mut()
                    .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
                response
            }),
        )
        .layer(middleware::from_fn_with_state(generation, inject))
}

async fn inject(State(generation): State<Arc<str>>, mut request: Request, next: Next) -> Response {
    use axum::body::HttpBody;
    // An HTMX fragment belongs to a document whose singleton poller is already
    // running. Adding executable scripts to each swap accumulates poll loops.
    let partial = request
        .headers()
        .get("hx-request")
        .is_some_and(|value| value.as_bytes().eq_ignore_ascii_case(b"true"));
    let head = request.method() == axum::http::Method::HEAD;
    // Seed before inner application header middleware, which reuses this nonce.
    let nonce = crate::security::CspNonce::get_or_insert(request.extensions_mut());
    let response = next.run(request).await;
    let html = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            v.split(';')
                .next()
                .is_some_and(|mime| mime.trim().eq_ignore_ascii_case("text/html"))
        });
    if partial
        || head
        || !html
        || response.status() == axum::http::StatusCode::PARTIAL_CONTENT
        || response.headers().contains_key(header::CONTENT_ENCODING)
        || !response
            .body()
            .size_hint()
            .upper()
            .is_some_and(|size| size <= LIMIT as u64)
    {
        return response;
    }
    let (mut parts, body) = response.into_parts();
    let Ok(bytes) = axum::body::to_bytes(body, LIMIT).await else {
        let mut response = Response::new(Body::from(
            "Development reload could not read the HTML response",
        ));
        *response.status_mut() = axum::http::StatusCode::INTERNAL_SERVER_ERROR;
        return response;
    };
    let Ok(mut html) = String::from_utf8(bytes.to_vec()) else {
        return Response::from_parts(parts, Body::from(bytes));
    };
    if !html.contains(PATH) {
        let nonce = format!(" nonce=\"{}\"", nonce.as_str());
        let script = format!(
            "<script src=\"{PATH}\" data-generation=\"{generation}\"{nonce} defer></script>"
        );
        let position = html.rfind("</body>").unwrap_or(html.len());
        html.insert_str(position, &script);
        parts.headers.remove(header::CONTENT_LENGTH);
        parts.headers.remove(header::ETAG);
        parts.headers.remove(header::LAST_MODIFIED);
        parts
            .headers
            .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }
    Response::from_parts(parts, Body::from(html))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use tower::ServiceExt;

    #[tokio::test]
    async fn inner_nonce_only_csp_matches_injected_script_and_mutated_html_is_not_cached() {
        let security = crate::config::SecurityConfig {
            csp: "default-src 'self'; script-src 'nonce-{NONCE}'".into(),
            ..Default::default()
        };
        let router = Router::new()
            .route(
                "/",
                get(|| async {
                    let mut response = axum::response::IntoResponse::into_response(
                        axum::response::Html("<body>page</body>"),
                    );
                    response.headers_mut().insert(
                        header::CACHE_CONTROL,
                        HeaderValue::from_static("public, max-age=600"),
                    );
                    response
                        .headers_mut()
                        .insert(header::ETAG, HeaderValue::from_static("\"old\""));
                    response.headers_mut().insert(
                        header::LAST_MODIFIED,
                        HeaderValue::from_static("Wed, 01 Jan 2025 00:00:00 GMT"),
                    );
                    response
                }),
            )
            .layer(middleware::from_fn(crate::security::headers_middleware))
            .layer(axum::Extension(security));
        let response = mount(
            router,
            true,
            Some("0123456789abcdef0123456789abcdef".into()),
        )
        .oneshot(Request::new(Body::empty()))
        .await
        .unwrap();
        if !cfg!(debug_assertions) {
            return;
        }
        let policy = response.headers()[header::CONTENT_SECURITY_POLICY]
            .to_str()
            .unwrap()
            .to_string();
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        assert!(!response.headers().contains_key(header::ETAG));
        assert!(!response.headers().contains_key(header::LAST_MODIFIED));
        let bytes = axum::body::to_bytes(response.into_body(), LIMIT)
            .await
            .unwrap();
        let html = std::str::from_utf8(&bytes).unwrap();
        let nonce = html
            .split("nonce=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap();
        assert!(policy.contains(&format!("'nonce-{nonce}'")));
    }

    #[tokio::test]
    async fn htmx_fragments_and_encoded_html_do_not_accumulate_reload_scripts() {
        let router = Router::new().route(
            "/",
            get(|| async { axum::response::Html("<div>fragment</div>") }),
        );
        let router = mount(
            router,
            true,
            Some("0123456789abcdef0123456789abcdef".into()),
        );
        for _ in 0..3 {
            let response = router
                .clone()
                .oneshot(
                    Request::builder()
                        .header("hx-request", "true")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            let bytes = axum::body::to_bytes(response.into_body(), LIMIT)
                .await
                .unwrap();
            assert_eq!(&bytes[..], b"<div>fragment</div>");
        }
        let router = Router::new().route(
            "/",
            get(|| async {
                let mut response = Response::new(Body::from("compressed bytes"));
                response
                    .headers_mut()
                    .insert(header::CONTENT_TYPE, HeaderValue::from_static("text/html"));
                response
                    .headers_mut()
                    .insert(header::CONTENT_ENCODING, HeaderValue::from_static("gzip"));
                response
            }),
        );
        let response = mount(
            router,
            true,
            Some("0123456789abcdef0123456789abcdef".into()),
        )
        .oneshot(Request::new(Body::empty()))
        .await
        .unwrap();
        assert_eq!(response.headers()[header::CONTENT_ENCODING], "gzip");
        let bytes = axum::body::to_bytes(response.into_body(), LIMIT)
            .await
            .unwrap();
        assert_eq!(&bytes[..], b"compressed bytes");
    }

    #[tokio::test]
    async fn unknown_length_html_streams_pass_through_without_buffering_or_injection() {
        use axum::body::HttpBody;
        let router = Router::new().route(
            "/",
            get(|| async {
                let stream = Body::from("<body>stream ends</body>").into_data_stream();
                let mut response = Response::new(Body::from_stream(stream));
                response
                    .headers_mut()
                    .insert(header::CONTENT_TYPE, HeaderValue::from_static("text/html"));
                response
            }),
        );
        let response = mount(
            router,
            true,
            Some("0123456789abcdef0123456789abcdef".into()),
        )
        .oneshot(Request::new(Body::empty()))
        .await
        .unwrap();
        assert_eq!(response.body().size_hint().upper(), None);
        let bytes = axum::body::to_bytes(response.into_body(), LIMIT)
            .await
            .unwrap();
        assert_eq!(&bytes[..], b"<body>stream ends</body>");
    }
    #[tokio::test]
    async fn polling_surface_requires_debug_development_and_valid_generation() {
        let marker = "0123456789abcdef0123456789abcdef";
        let router = || {
            Router::new().route(
                "/",
                get(|| async { axum::response::Html("<body>ready</body>") }),
            )
        };
        for (development, generation) in [
            (false, Some(marker.into())),
            (true, None),
            (true, Some("bad\"<marker>".into())),
        ] {
            let response = mount(router(), development, generation)
                .oneshot(
                    Request::builder()
                        .uri("/_rullst/dev-generation")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), 404);
        }
        let router = mount(router(), true, Some(marker.into()));
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/_rullst/dev-generation")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        if cfg!(debug_assertions) {
            assert_eq!(response.headers()["x-rullst-dev-generation"], marker);
            assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
            let response = router
                .clone()
                .oneshot(Request::new(Body::empty()))
                .await
                .unwrap();
            let body = axum::body::to_bytes(response.into_body(), LIMIT)
                .await
                .unwrap();
            let html = String::from_utf8(body.to_vec()).unwrap();
            assert!(html.contains(&format!("data-generation=\"{marker}\"")));
            assert!(!html.contains("unpkg.com"));
        } else {
            assert_eq!(response.status(), 404);
        }
        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/_rullst/dev-generation")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(!response.status().is_success());
    }
}
