/// Intercepts HTML responses in development mode and injects morphdom WebSocket HMR script.
#[cfg_attr(mutants, mutants::skip)]
pub async fn inject_hmr_script(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let res = next.run(req).await;

    if let Some(content_type) = res.headers().get(axum::http::header::CONTENT_TYPE) {
        if content_type.to_str().unwrap_or("").contains("text/html") {
            let (mut parts, body) = res.into_parts();
            if let Ok(bytes) = axum::body::to_bytes(body, usize::MAX).await {
                let mut html = String::from_utf8_lossy(&bytes).to_string();

                let port = std::env::var("PORT")
                    .ok()
                    .and_then(|p| p.parse::<u16>().ok())
                    .unwrap_or(3000);
                let ws_port = port + 1;

                let script = format!(
                    r#"
<!-- Rullst Hybrid Hot-Reloading -->
<script src="https://unpkg.com/morphdom@2.7.4/dist/morphdom-umd.js"></script>
<script>
    (function connectHmr() {{
        const host = window.location.hostname || '127.0.0.1';
        const ws = new WebSocket(`ws://${{host}}:{}/_rullst_hmr`);
        ws.onmessage = (e) => {{
            const data = JSON.parse(e.data);
            if (data.type === "UI_UPDATE") {{
                fetch(window.location.href)
                    .then(r => r.text())
                    .then(newHtml => {{
                        const parser = new DOMParser();
                        const doc = parser.parseFromString(newHtml, 'text/html');
                        morphdom(document.body, doc.body, {{
                            onBeforeElUpdated: function(fromEl, toEl) {{
                                if (fromEl.isEqualNode(toEl)) return false;
                                return true;
                            }}
                        }});
                    }});
            }}
        }};
        ws.onclose = () => setTimeout(connectHmr, 1000);
    }})();
</script>
"#,
                    ws_port
                );
                if let Some(idx) = html.rfind("</body>") {
                    html.insert_str(idx, &script);
                } else {
                    html.push_str(&script);
                }

                parts.headers.remove(axum::http::header::CONTENT_LENGTH);
                return axum::response::Response::from_parts(parts, axum::body::Body::from(html));
            } else {
                return axum::response::Response::from_parts(parts, axum::body::Body::empty());
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

/// Adds standard W3C `Server-Timing` headers to HTTP responses for instant DevTools network profiling.
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
