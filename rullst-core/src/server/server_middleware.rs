const HMR_CLIENT_PATH: &str = "/_rullst/hmr-client.js";
const MAX_HMR_BODY_BYTES: usize = 10 * 1024 * 1024;
const HMR_CLIENT: &str = r#"(() => {
    'use strict';
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
#[cfg_attr(mutants, mutants::skip)]
pub async fn inject_hmr_script(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::body::HttpBody as _;

    let res = next.run(req).await;

    if let Some(content_type) = res.headers().get(axum::http::header::CONTENT_TYPE) {
        if content_type.to_str().unwrap_or("").contains("text/html") {
            let declared_too_large = res
                .headers()
                .get(axum::http::header::CONTENT_LENGTH)
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
            if let Ok(bytes) = axum::body::to_bytes(body, MAX_HMR_BODY_BYTES).await {
                let mut html = String::from_utf8_lossy(&bytes).to_string();

                if !html.contains(HMR_CLIENT_PATH) {
                    let script = format!(
                        "\n<!-- Rullst authenticated local hot reload -->\n<script src=\"{HMR_CLIENT_PATH}\" defer></script>\n"
                    );
                    if let Some(idx) = html.rfind("</body>") {
                        html.insert_str(idx, &script);
                    } else {
                        html.push_str(&script);
                    }
                }

                parts.headers.remove(axum::http::header::CONTENT_LENGTH);
                return axum::response::Response::from_parts(parts, axum::body::Body::from(html));
            } else {
                eprintln!(
                    "Rullst HMR could not buffer a bounded development HTML response; returning an explicit error."
                );
                let mut response = axum::response::Response::new(axum::body::Body::from(
                    "Rullst HMR could not read the development HTML response.",
                ));
                *response.status_mut() = axum::http::StatusCode::INTERNAL_SERVER_ERROR;
                response.headers_mut().insert(
                    axum::http::header::CONTENT_TYPE,
                    axum::http::HeaderValue::from_static("text/plain; charset=utf-8"),
                );
                return response;
            }
        }
    }

    res
}

/// Serves `.zst` compressed static assets when requested with matching `Accept-Encoding`.
#[cfg_attr(mutants, mutants::skip)]
pub async fn zstd_static_middleware(
    mut req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let path = req.uri().path().to_string();
    if path.starts_with("/static/") {
        if let Some(accept_encoding) = req.headers().get(axum::http::header::ACCEPT_ENCODING) {
            if let Ok(accept_str) = accept_encoding.to_str() {
                if accept_str.contains("zstd") {
                    let local_path_str = format!("{}.zst", &path[1..]);
                    if tokio::fs::metadata(&local_path_str)
                        .await
                        .map(|m| m.is_file())
                        .unwrap_or(false)
                    {
                        let original_ext = std::path::Path::new(&path)
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .unwrap_or("")
                            .to_string();

                        let new_uri = format!("{}.zst", path);
                        if let Ok(uri) = new_uri.parse::<axum::http::Uri>() {
                            *req.uri_mut() = uri;

                            let mut response = next.run(req).await;

                            response.headers_mut().insert(
                                axum::http::header::CONTENT_ENCODING,
                                axum::http::header::HeaderValue::from_static("zstd"),
                            );

                            let mime_type = match original_ext.as_str() {
                                "html" => "text/html; charset=utf-8",
                                "css" => "text/css; charset=utf-8",
                                "js" => "application/javascript; charset=utf-8",
                                "json" => "application/json; charset=utf-8",
                                "svg" => "image/svg+xml",
                                "wasm" => "application/wasm",
                                "xml" => "application/xml; charset=utf-8",
                                "txt" => "text/plain; charset=utf-8",
                                _ => "",
                            };

                            if !mime_type.is_empty() {
                                if let Ok(val) =
                                    axum::http::header::HeaderValue::from_str(mime_type)
                                {
                                    response
                                        .headers_mut()
                                        .insert(axum::http::header::CONTENT_TYPE, val);
                                }
                            }

                            return response;
                        }
                    }
                }
            }
        }
    }

    next.run(req).await
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
