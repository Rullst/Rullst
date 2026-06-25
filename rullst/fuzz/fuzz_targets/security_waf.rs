#![no_main]
use libfuzzer_sys::fuzz_target;
use axum::{http::Request, Router, routing::get, middleware::from_fn};
use rullst::security::waf_middleware;
use tower::ServiceExt;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(async {
            let app = Router::new()
                .route("/", get(|| async { "OK" }))
                .route_layer(from_fn(waf_middleware));
                
            let uri = format!("/?q={}", urlencoding::encode(s));
            if let Ok(req) = Request::builder().uri(&uri).body(axum::body::Body::empty()) {
                let _ = app.oneshot(req).await;
            }
        });
    }
});
