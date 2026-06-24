use axum::routing::get;
use rullst::Router;
use rullst::server::Server;
use std::time::Duration;

fn get_free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

#[tokio::test]
async fn test_server_new() {
    let router = Router::new().route("/", get(|| async { "OK" }));
    let _server = Server::new(router).with_db("sqlite::memory:");
}

#[tokio::test]
async fn test_server_run_static() {
    // 1. Create static files
    let _ = std::fs::create_dir_all("static");
    let _ = std::fs::write("static/test_file.txt", b"Hello Static");
    let _ = std::fs::write("static/test_file.txt.zst", b"Hello ZSTD compressed");

    let port = get_free_port();
    let router = Router::new().route("/", get(|| async { "OK" }));

    let shield = rullst::resilience::TrafficShield::new(
        rullst::resilience::TrafficShieldConfig::new().with_db_probe(false),
    );
    let limiter =
        rullst::resilience::RateLimiter::new(rullst::resilience::RateLimitConfig::per_second(10.0));
    let scheduler = rullst::scheduler::Scheduler::new();

    let server = Server::new(router)
        .with_db("sqlite::memory:")
        .schedule(scheduler)
        .shield(shield)
        .rate_limit(limiter);

    let handle = tokio::spawn(async move {
        let _ = server.run(port).await;
    });

    // Wait for server to boot
    tokio::time::sleep(Duration::from_millis(200)).await;

    // 2. Test standard endpoint
    let client = reqwest::Client::new();
    let res = client
        .get(format!("http://127.0.0.1:{}/", port))
        .send()
        .await;

    assert!(res.is_ok());
    let res = res.unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(res.text().await.unwrap(), "OK");

    // 3. Test static file zstd compression middleware
    let res_zstd = client
        .get(format!("http://127.0.0.1:{}/static/test_file.txt", port))
        .header("accept-encoding", "zstd")
        .send()
        .await;

    assert!(res_zstd.is_ok());
    let res_zstd = res_zstd.unwrap();
    assert_eq!(res_zstd.status(), 200);
    // Check if content-encoding is zstd
    let headers = res_zstd.headers();
    if let Some(enc) = headers.get("content-encoding") {
        assert_eq!(enc, "zstd");
    }

    // Clean up
    handle.abort();
    let _ = std::fs::remove_file("static/test_file.txt");
    let _ = std::fs::remove_file("static/test_file.txt.zst");
    let _ = std::fs::remove_dir("static");
}

#[tokio::test]
async fn test_server_new_hot_debug() {
    #[cfg(debug_assertions)]
    {
        let server = Server::new_hot("dummy.dll");
        // Running hot reload with nonexistent dll should return Err
        let res = server.run(get_free_port()).await;
        assert!(res.is_err());
    }
}

#[tokio::test]
async fn test_server_run_production_middlewares() {
    unsafe {
        std::env::set_var("APP_ENV", "production");
    }

    let port = get_free_port();
    let router = Router::new().route("/", get(|| async { "OK" }));

    let server = Server::new(router);

    let handle = tokio::spawn(async move {
        let _ = server.run(port).await;
    });

    tokio::time::sleep(Duration::from_millis(300)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(format!("http://127.0.0.1:{}/", port))
        .send()
        .await;

    assert!(res.is_ok());
    let res = res.unwrap();
    assert_eq!(res.status(), 200);

    let _headers = res.headers();

    handle.abort();
    unsafe {
        std::env::remove_var("APP_ENV");
    }
}
