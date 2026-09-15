//! Regressions for the v12 Portfolio report: raw browser CSS and Cache links.
//! Authentication below is a layering fixture, not a production login policy.

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
    response::Response,
};
use rullst_studio::data_browser;
use tower::ServiceExt;

fn request(path: &str) -> Request<Body> {
    Request::builder().uri(path).body(Body::empty()).unwrap()
}

async fn body(response: Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), 512 * 1024)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

fn browsers() -> [Router; 2] {
    [
        data_browser::router(),
        Router::new().nest("/studio", data_browser::router()),
    ]
}

#[tokio::test]
async fn raw_and_nested_browsers_serve_the_layout_stylesheet_and_client() {
    for app in browsers() {
        for (name, content_type) in [
            ("studio.css", "text/css; charset=utf-8"),
            ("logger.js", "application/javascript; charset=utf-8"),
        ] {
            let path = format!("/studio/assets/{name}");
            let response = app.clone().oneshot(request(&path)).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{path}");
            assert_eq!(response.headers()[header::CONTENT_TYPE], content_type);
            assert_eq!(
                response.headers()[header::X_CONTENT_TYPE_OPTIONS],
                "nosniff"
            );
            assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
            let text = body(response).await;
            if name == "studio.css" {
                assert_eq!(text, include_str!("../assets/studio.css"));
            } else {
                assert!(text.contains("new EventSource(\"/studio/requests/stream\")"));
            }
            assert!(!text.contains("cdn.tailwindcss.com"));

            let mut head = request(&path);
            *head.method_mut() = axum::http::Method::HEAD;
            let response = app.clone().oneshot(head).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            assert!(body(response).await.is_empty());

            let mut post = request(&path);
            *post.method_mut() = axum::http::Method::POST;
            assert_eq!(
                app.clone().oneshot(post).await.unwrap().status(),
                StatusCode::METHOD_NOT_ALLOWED
            );
        }
        assert_eq!(
            app.oneshot(request("/studio/assets/not-an-asset.css"))
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
    }

    // Unprefixed aliases are consumed by Axum when nested under /studio.
    for path in ["/assets/studio.css", "/assets/logger.js"] {
        assert_eq!(
            data_browser::router()
                .oneshot(request(path))
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
    }
}

#[tokio::test]
async fn raw_cache_navigation_is_honestly_unavailable_for_full_and_partial_visits() {
    for app in browsers() {
        for partial in [false, true] {
            let mut req = request("/studio/cache");
            if partial {
                req.headers_mut()
                    .insert("hx-request", "true".parse().unwrap());
            }
            let response = app.clone().oneshot(req).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let html = body(response).await;
            assert!(html.contains("Cache Inspector"));
            assert!(html.contains("Unavailable"));
            assert!(html.contains("Studio::with_cache"));
            assert!(!html.contains("<form"));
            assert!(!html.contains("No live cache entries"));
            assert_eq!(html.contains("<!DOCTYPE html>"), !partial);
            if !partial {
                let marker = "href=\"/studio/assets/";
                let start = html.find(marker).expect("same-origin stylesheet") + "href=\"".len();
                let href = html[start..].split('"').next().unwrap();
                assert_eq!(
                    app.clone().oneshot(request(href)).await.unwrap().status(),
                    StatusCode::OK,
                    "stylesheet actually referenced by the returned page"
                );
            }
        }
        for path in ["/studio/cache/forget", "/studio/cache/flush"] {
            let mut req = request(path);
            *req.method_mut() = axum::http::Method::POST;
            assert_eq!(
                app.clone().oneshot(req).await.unwrap().status(),
                StatusCode::NOT_FOUND,
                "the unconnected browser must not expose cache mutations"
            );
        }
    }
    assert_eq!(
        data_browser::router()
            .oneshot(request("/cache"))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
}

#[tokio::test]
async fn new_browser_routes_remain_inside_the_callers_access_layer() {
    async fn fixture_guard(req: axum::extract::Request, next: axum::middleware::Next) -> Response {
        use axum::response::IntoResponse;
        if req.headers().get("x-test-admitted").is_none() {
            return StatusCode::UNAUTHORIZED.into_response();
        }
        next.run(req).await
    }
    let app = Router::new().nest(
        "/studio",
        data_browser::router().layer(axum::middleware::from_fn(fixture_guard)),
    );
    for path in [
        "/studio/assets/studio.css",
        "/studio/assets/logger.js",
        "/studio/cache",
    ] {
        assert_eq!(
            app.clone().oneshot(request(path)).await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );
        let mut admitted = request(path);
        admitted
            .headers_mut()
            .insert("x-test-admitted", "true".parse().unwrap());
        assert_eq!(
            app.clone().oneshot(admitted).await.unwrap().status(),
            StatusCode::OK
        );
    }
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn full_studio_keeps_real_cache_and_local_access_without_route_collisions() {
    use rullst_studio::{LocalStudioAccess, Studio};
    let cache = rullst_core::Cache::memory();
    cache
        .put("private:key", "private-value", None)
        .await
        .unwrap();
    for configured in [false, true] {
        let studio = if configured {
            Studio::new().with_cache(cache.clone())
        } else {
            Studio::new()
        };
        let app = studio
            .into_router(LocalStudioAccess::loopback_only())
            .expect("no duplicate route registration");
        for path in ["/studio/assets/studio.css", "/studio/cache"] {
            for loopback in [false, true] {
                let mut req = request(path);
                req.headers_mut()
                    .insert(header::HOST, "127.0.0.1:5555".parse().unwrap());
                let peer: std::net::SocketAddr = if loopback {
                    "127.0.0.1:42000"
                } else {
                    "192.0.2.1:42000"
                }
                .parse()
                .unwrap();
                req.extensions_mut()
                    .insert(axum::extract::ConnectInfo(peer));
                let response = app.clone().oneshot(req).await.unwrap();
                if !loopback {
                    assert_eq!(response.status(), StatusCode::FORBIDDEN);
                    continue;
                }
                assert_eq!(response.status(), StatusCode::OK);
                if path.ends_with("/cache") {
                    let html = body(response).await;
                    assert_eq!(html.contains("13 bytes"), configured);
                    assert_eq!(html.contains("Unavailable"), !configured);
                    assert!(!html.contains("private:key"));
                    assert!(!html.contains("private-value"));
                }
            }
        }
    }
}
