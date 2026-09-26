use rullst_core::{
    http::Request,
    server::{Body, StatusCode},
    web::axum::body::to_bytes,
};
use tower::ServiceExt;

#[tokio::test]
async fn embedded_assets_keep_the_production_security_baseline() {
    let app = rullst_webgpu_example::router().unwrap();
    for (path, mime) in [
        ("/webgpu/", "text/html"),
        ("/webgpu/style.css", "text/css"),
        ("/webgpu/app.mjs", "text/javascript"),
        ("/webgpu/controller.mjs", "text/javascript"),
        ("/webgpu/waves.mjs", "text/javascript"),
        ("/webgpu/gpu.mjs", "text/javascript"),
    ] {
        let response = app
            .clone()
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        let headers = response.headers();
        assert!(headers["content-type"].to_str().unwrap().starts_with(mime));
        assert_eq!(headers["cache-control"], "no-store");
        assert_eq!(headers["x-content-type-options"], "nosniff");
        let csp = headers["content-security-policy"].to_str().unwrap();
        assert!(csp.contains("script-src 'self'"));
        assert!(csp.contains("frame-ancestors 'none'"));
        assert!(!csp.contains("unsafe-inline"));
        assert!(!csp.contains("unsafe-eval"));
        assert!(
            to_bytes(response.into_body(), 64 * 1024)
                .await
                .unwrap()
                .len()
                > 50
        );
    }
    for path in [
        "/webgpu/Cargo.toml",
        "/webgpu/src/main.rs",
        "/webgpu/missing.mjs",
    ] {
        let response = app
            .clone()
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
    let response = app
        .oneshot(Request::post("/webgpu/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert!(matches!(
        response.status(),
        StatusCode::FORBIDDEN | StatusCode::METHOD_NOT_ALLOWED
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_acceptance_against_the_rullst_server() {
    if std::env::var("RULLST_WEBGPU_BROWSER_TESTS").as_deref() != Ok("1") {
        eprintln!("WebGPU browser acceptance runs in the Linux CI workspace shard.");
        return;
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        rullst_core::web::axum::serve(listener, rullst_webgpu_example::router().unwrap())
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    });
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.github/webgpu-browser-smoke.mjs");
    let result = tokio::task::spawn_blocking(move || {
        std::process::Command::new("node")
            .arg(script)
            .arg(origin)
            .output()
            .unwrap()
    })
    .await
    .unwrap();
    let _ = stop.send(());
    server.await.unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    println!("{}", String::from_utf8_lossy(&result.stdout));
}
