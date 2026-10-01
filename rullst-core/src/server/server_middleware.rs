const HMR_CLIENT_PATH: &str = "/_rullst/hmr-client.js";
const MAX_HMR_BODY_BYTES: usize = 10 * 1024 * 1024;
const HMR_CLIENT: &str = r#"(() => {
    'use strict';
    if (window.__rullstHmr) return;
    window.__rullstHmr = true;
    let retryDelayMs = 250;
    const connect = () => {
        const socketProtocol = window.location.protocol === 'https:' ? 'wss' : 'ws';
        const socket = new WebSocket(`${socketProtocol}://${window.location.host}/_rullst_hmr`);
        socket.onopen = () => { retryDelayMs = 250; };
        socket.onmessage = (event) => {
            try {
                const message = JSON.parse(event.data);
                if (message.type === 'UI_UPDATE') window.location.reload();
            } catch (_) {
                console.warn('Rullst HMR ignored a malformed local message.');
            }
        };
        socket.onclose = () => {
            window.setTimeout(connect, retryDelayMs);
            retryDelayMs = Math.min(retryDelayMs * 2, 5000);
        };
    };
    connect();
})();
"#;

/// Serves the same-origin, offline hot-reload browser client in development.
pub(crate) async fn hmr_client_script() -> axum::response::Response {
    let mut response = axum::response::Response::new(axum::body::Body::from(HMR_CLIENT));
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/javascript; charset=utf-8"),
    );
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    response
}

/// Intercepts development HTML responses and injects the local HMR client.
///
/// Only complete, uncompressed UTF-8 `text/html` documents are changed: HTMX
/// fragment requests (`HX-Request: true`), `HEAD`, `206 Partial Content`,
/// responses with a `Content-Encoding`, bodies above 10 MiB and non-UTF-8
/// bodies pass through untouched. The injected script carries the request's
/// CSP nonce, which inner security header middleware reuses, and a changed
/// page drops its `ETag` and `Last-Modified` validators.
#[cfg_attr(mutants, mutants::skip)]
pub async fn inject_hmr_script(
    mut req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::body::HttpBody as _;
    use axum::http::header;

    // An HTMX swap belongs to a page whose client is already connected.
    let fragment = req
        .headers()
        .get("hx-request")
        .is_some_and(|value| value.as_bytes().eq_ignore_ascii_case(b"true"));
    let head = req.method() == axum::http::Method::HEAD;
    let nonce = crate::security::CspNonce::get_or_insert(req.extensions_mut());

    let res = next.run(req).await;

    let is_html = res
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|mime| mime.trim().eq_ignore_ascii_case("text/html"));
    if fragment
        || head
        || !is_html
        || res.status() == axum::http::StatusCode::PARTIAL_CONTENT
        || res.headers().contains_key(header::CONTENT_ENCODING)
    {
        return res;
    }
    let declared_too_large = res
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .is_some_and(|length| length > MAX_HMR_BODY_BYTES as u64);
    let body_is_bounded = res
        .body()
        .size_hint()
        .upper()
        .is_some_and(|length| length <= MAX_HMR_BODY_BYTES as u64);
    if declared_too_large || !body_is_bounded {
        return res;
    }

    let (mut parts, body) = res.into_parts();
    let Ok(bytes) = axum::body::to_bytes(body, MAX_HMR_BODY_BYTES).await else {
        crate::server::console::stderr_line(format_args!(
            "Rullst HMR could not buffer a bounded development HTML response; returning an explicit error."
        ));
        let mut response = axum::response::Response::new(axum::body::Body::from(
            "Rullst HMR could not read the development HTML response.",
        ));
        *response.status_mut() = axum::http::StatusCode::INTERNAL_SERVER_ERROR;
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("text/plain; charset=utf-8"),
        );
        return response;
    };
    let Ok(mut html) = String::from_utf8(bytes.to_vec()) else {
        return axum::response::Response::from_parts(parts, axum::body::Body::from(bytes));
    };
    if !html.contains(HMR_CLIENT_PATH) {
        let script = format!(
            "\n<!-- Rullst authenticated local hot reload -->\n<script src=\"{HMR_CLIENT_PATH}\" nonce=\"{}\" defer></script>\n",
            nonce.as_str()
        );
        let position = html.rfind("</body>").unwrap_or(html.len());
        html.insert_str(position, &script);
        parts.headers.remove(header::CONTENT_LENGTH);
        parts.headers.remove(header::ETAG);
        parts.headers.remove(header::LAST_MODIFIED);
    }
    axum::response::Response::from_parts(parts, axum::body::Body::from(html))
}

/// Serves `.zst` compressed static assets when requested with matching `Accept-Encoding`.
///
/// The variant is used only when `Accept-Encoding` lists `zstd` with a
/// non-zero quality (`zstd;q=0` is a refusal) and the path below `/static/`
/// contains only plain segments: no `.`/`..`/empty segment, backslash or
/// percent-encoding, so nothing outside `static/` is ever probed. Such paths
/// fall through to the uncompressed service unchanged. `Content-Encoding` and
/// the original `Content-Type` are set only on a successful (2xx) or `304`
/// response. Only types this middleware can label are served from `.zst`
/// (HTML, CSS, JavaScript including `.mjs`/`.cjs`, JSON and source maps, web
/// manifests, SVG, Wasm, XML, text, CSV, PDF, TTF and OTF); any other file is
/// served uncompressed rather than as `application/octet-stream`.
#[cfg_attr(mutants, mutants::skip)]
pub async fn zstd_static_middleware(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    zstd_static_from(std::path::Path::new("."), req, next).await
}

pub(crate) async fn zstd_static_from(
    root: &std::path::Path,
    mut req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let Some(relative) = static_asset_path(req.uri().path()).map(str::to_string) else {
        return next.run(req).await;
    };
    if !accepts_zstd(req.headers()) {
        return next.run(req).await;
    }
    // `ServeDir` would label `<file>.zst` as application/octet-stream; serve
    // the uncompressed file when the original type is not known here.
    let Some(mime_type) = zstd_content_type(&relative) else {
        return next.run(req).await;
    };
    let compressed = root.join("static").join(format!("{relative}.zst"));
    let exists = tokio::fs::metadata(&compressed)
        .await
        .is_ok_and(|metadata| metadata.is_file());
    if !exists {
        return next.run(req).await;
    }
    let Ok(uri) = format!("/static/{relative}.zst").parse::<axum::http::Uri>() else {
        return next.run(req).await;
    };
    *req.uri_mut() = uri;

    let mut response = next.run(req).await;
    let status = response.status();
    if !(status.is_success() || status == axum::http::StatusCode::NOT_MODIFIED) {
        return response;
    }
    response.headers_mut().insert(
        axum::http::header::CONTENT_ENCODING,
        axum::http::header::HeaderValue::from_static("zstd"),
    );
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::header::HeaderValue::from_static(mime_type),
    );
    response
}

/// The `Content-Type` restored on a `.zst` variant of `relative`, or `None`
/// for a type this middleware does not know (served uncompressed instead).
fn zstd_content_type(relative: &str) -> Option<&'static str> {
    let extension = std::path::Path::new(relative)
        .extension()
        .and_then(|ext| ext.to_str())?
        .to_ascii_lowercase();
    Some(match extension.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" | "cjs" => "application/javascript; charset=utf-8",
        "json" | "map" => "application/json; charset=utf-8",
        "webmanifest" => "application/manifest+json; charset=utf-8",
        "svg" => "image/svg+xml",
        "wasm" => "application/wasm",
        "xml" => "application/xml; charset=utf-8",
        "txt" => "text/plain; charset=utf-8",
        "csv" => "text/csv; charset=utf-8",
        "pdf" => "application/pdf",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        _ => return None,
    })
}

/// Returns the part of a `/static/...` request path that may be probed on
/// disk, or `None` when any segment is empty, `.`, `..`, or contains a
/// backslash or `%` (ServeDir percent-decodes, so a raw probe could differ).
pub(crate) fn static_asset_path(path: &str) -> Option<&str> {
    let relative = path.strip_prefix("/static/")?;
    let plain = !relative.is_empty()
        && !relative.contains(['\\', '%', '\0'])
        && relative
            .split('/')
            .all(|segment| !matches!(segment, "" | "." | ".."));
    plain.then_some(relative)
}

/// Whether `Accept-Encoding` lists `zstd` with a quality above zero.
pub(crate) fn accepts_zstd(headers: &axum::http::HeaderMap) -> bool {
    headers
        .get_all(axum::http::header::ACCEPT_ENCODING)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .any(|item| {
            let mut parts = item.split(';');
            let coding = parts.next().unwrap_or("").trim();
            if !coding.eq_ignore_ascii_case("zstd") {
                return false;
            }
            let quality = parts.find_map(|param| {
                let (name, value) = param.split_once('=')?;
                name.trim()
                    .eq_ignore_ascii_case("q")
                    .then(|| value.trim().parse::<f32>().ok())
            });
            match quality {
                None => true,
                Some(Some(q)) => q > 0.0 && q <= 1.0,
                Some(None) => false,
            }
        })
}

/// Adds a standard W3C `Server-Timing` header with the observed handler duration.
///
/// Format: `Server-Timing: app;dur=X.XX;desc="Rullst App Handler"`
#[cfg_attr(mutants, mutants::skip)]
pub async fn server_timing_middleware(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let start = std::time::Instant::now();
    let mut res = next.run(req).await;
    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

    let timing_header_val = format!("app;dur={:.2};desc=\"Rullst App Handler\"", elapsed_ms);
    if let Ok(val) = axum::http::HeaderValue::from_str(&timing_header_val) {
        res.headers_mut().insert(
            axum::http::header::HeaderName::from_static("server-timing"),
            val,
        );
    }

    res
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use tower::ServiceExt;

    fn accept(value: &str) -> axum::http::HeaderMap {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            axum::http::header::ACCEPT_ENCODING,
            axum::http::HeaderValue::from_str(value).unwrap(),
        );
        headers
    }

    #[test]
    fn zstd_requires_a_positive_quality() {
        for value in ["zstd", "gzip, zstd", "ZSTD;q=0.5", "br, zstd ; q=1"] {
            assert!(accepts_zstd(&accept(value)), "{value}");
        }
        for value in [
            "gzip, zstd;q=0",
            "zstd;q=0.000",
            "zstd;q=abc",
            "zstd;q=2",
            "gzip",
            "xzstd",
        ] {
            assert!(!accepts_zstd(&accept(value)), "{value}");
        }
        assert!(!accepts_zstd(&axum::http::HeaderMap::new()));
    }

    #[test]
    fn only_plain_static_paths_are_probed() {
        assert_eq!(static_asset_path("/static/app.js"), Some("app.js"));
        assert_eq!(
            static_asset_path("/static/css/site.css"),
            Some("css/site.css")
        );
        for path in [
            "/static/",
            "/static/../../etc/backup.sql",
            "/static/a/../../b",
            "/static/./app.js",
            "/static//app.js",
            "/static/%2e%2e/secret",
            "/static/a\\..\\b",
            "/other/app.js",
        ] {
            assert_eq!(static_asset_path(path), None, "{path}");
        }
    }

    async fn get(router: &axum::Router, uri: &str, encoding: &str) -> axum::response::Response {
        router
            .clone()
            .oneshot(
                axum::http::Request::get(uri)
                    .header(axum::http::header::ACCEPT_ENCODING, encoding)
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn zstd_variant_honours_refusals_and_failed_responses() {
        struct TempRoot(std::path::PathBuf);
        impl Drop for TempRoot {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let dir = TempRoot(
            std::env::temp_dir().join(format!("rullst-zstd-{}", uuid::Uuid::new_v4().simple())),
        );
        let root = dir.0.clone();
        std::fs::create_dir_all(root.join("static")).unwrap();
        std::fs::write(root.join("static/app.js"), "plain").unwrap();
        std::fs::write(root.join("static/app.js.zst"), "compressed").unwrap();

        let layer_root = root.clone();
        let router = axum::Router::new()
            .nest_service(
                "/static",
                tower_http::services::ServeDir::new(root.join("static")),
            )
            .layer(axum::middleware::from_fn(move |req, next| {
                let root = layer_root.clone();
                async move { zstd_static_from(&root, req, next).await }
            }));

        let response = get(&router, "/static/app.js", "zstd").await;
        assert_eq!(response.status(), 200);
        assert_eq!(response.headers()["content-encoding"], "zstd");
        assert_eq!(
            response.headers()["content-type"],
            "application/javascript; charset=utf-8"
        );

        let response = get(&router, "/static/app.js", "gzip, zstd;q=0").await;
        assert_eq!(response.status(), 200);
        assert!(!response.headers().contains_key("content-encoding"));
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        assert_eq!(&body[..], b"plain");

        // A `.zst` file exists, but the inner service fails: no encoding header.
        let failing_root = root.clone();
        let failing = axum::Router::new()
            .fallback(|| async { axum::http::StatusCode::NOT_FOUND })
            .layer(axum::middleware::from_fn(move |req, next| {
                let root = failing_root.clone();
                async move { zstd_static_from(&root, req, next).await }
            }));
        let response = get(&failing, "/static/app.js", "zstd").await;
        assert_eq!(response.status(), 404);
        assert!(!response.headers().contains_key("content-encoding"));

        // Module scripts keep a JavaScript type; an unknown type is served
        // uncompressed instead of as application/octet-stream.
        std::fs::write(root.join("static/app.mjs"), "plain module").unwrap();
        std::fs::write(root.join("static/app.mjs.zst"), "compressed module").unwrap();
        std::fs::write(root.join("static/data.bin"), "plain bytes").unwrap();
        std::fs::write(root.join("static/data.bin.zst"), "compressed bytes").unwrap();
        let response = get(&router, "/static/app.mjs", "zstd").await;
        assert_eq!(response.headers()["content-encoding"], "zstd");
        assert_eq!(
            response.headers()["content-type"],
            "application/javascript; charset=utf-8"
        );
        let response = get(&router, "/static/data.bin", "zstd").await;
        assert!(!response.headers().contains_key("content-encoding"));
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        assert_eq!(&body[..], b"plain bytes");
    }
}

#[cfg(test)]
#[path = "hmr_injection_tests.rs"]
mod hmr_injection_tests;
