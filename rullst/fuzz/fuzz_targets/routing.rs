#![no_main]

use libfuzzer_sys::fuzz_target;
use axum::http::Request;
use axum::body::Body;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let router = rullst::routes![
            rullst::routing::get("/" => || async { "home" }),
            rullst::routing::post("/api/:id" => || async { "api" }),
            rullst::routing::get("/files/*path" => || async { "files" }),
        ];
        
        let app = router.into_axum();
        
        if let Ok(req) = Request::builder().uri(s).body(Body::empty()) {
            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
            let _ = rt.block_on(async {
                use tower::ServiceExt;
                let _ = app.oneshot(req).await;
            });
        }
    }
});
