use axum::routing::{get, post};
use rullst::Router;
use rullst::server::Server;
use std::ffi::OsString;
use std::time::Duration;
use tokio::sync::Mutex;

/// `Server::run` resolves its environment from `RULLST_ENV` and `APP_ENV`
/// when it starts. Every test in this binary holds this lock, so a production
/// override never reaches a sibling server and no thread reads the process
/// environment while one is being changed.
static SERVER_ENV_LOCK: Mutex<()> = Mutex::const_new(());

/// Selects the production environment for servers started while it lives and
/// restores the previous selectors when dropped, including on test failure.
struct ProductionEnvironment(Vec<(&'static str, Option<OsString>)>);

impl ProductionEnvironment {
    /// The caller must hold [`SERVER_ENV_LOCK`].
    fn select() -> Self {
        let saved = ["RULLST_ENV", "APP_ENV"]
            .into_iter()
            .map(|name| (name, std::env::var_os(name)))
            .collect();
        // `RULLST_ENV` takes precedence over `APP_ENV`, so set it explicitly.
        unsafe {
            std::env::set_var("RULLST_ENV", "production");
            std::env::remove_var("APP_ENV");
        }
        Self(saved)
    }
}

impl Drop for ProductionEnvironment {
    fn drop(&mut self) {
        for (name, value) in &self.0 {
            match value {
                Some(value) => unsafe { std::env::set_var(name, value) },
                None => unsafe { std::env::remove_var(name) },
            }
        }
    }
}

fn get_free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

async fn wait_until_listening(port: u16) {
    for _ in 0..200 {
        if tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .is_ok()
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("the server did not start listening on port {port}");
}

#[tokio::test]
async fn test_server_new() {
    let _lock = SERVER_ENV_LOCK.lock().await;
    let router = Router::new().route("/", get(|| async { "OK" }));
    let _server = Server::new(router).with_db("sqlite::memory:");
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_server_run_static() {
    let _lock = SERVER_ENV_LOCK.lock().await;
    // 1. Create static files
    let _ = std::fs::create_dir_all("static");
    let _ = std::fs::write("static/test_file.txt", b"Hello Static");
    let _ = std::fs::write("static/test_file.txt.zst", b"Hello ZSTD compressed");

    let port = get_free_port();
    let router = Router::new().route("/", get(|| async { "OK" }));

    let shield = rullst::resilience::TrafficShield::new(
        rullst::resilience::TrafficShieldConfig::new()
            .with_db_probe(false)
            .with_max_event_loop_lag(Duration::from_secs(10)),
    );
    let limiter =
        rullst::resilience::RateLimiter::new(rullst::resilience::RateLimitConfig::per_second(10.0));
    let scheduler = rullst::scheduler::Scheduler::new();

    let server = Server::new(router)
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
    let _lock = SERVER_ENV_LOCK.lock().await;
    #[cfg(debug_assertions)]
    {
        let server = Server::new_hot("dummy.dll");
        // Running hot reload with nonexistent dll should return Err
        let res = server.run(get_free_port()).await;
        assert!(res.is_err());
    }
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_server_run_production_middlewares() {
    let _lock = SERVER_ENV_LOCK.lock().await;
    let _production = ProductionEnvironment::select();

    let port = get_free_port();
    let router = Router::new()
        .route("/", get(|| async { "OK" }))
        .route("/write", post(|| async { "written" }));
    let handle = tokio::spawn(async move {
        let _ = Server::new(router).run(port).await;
    });
    wait_until_listening(port).await;

    let client = reqwest::Client::new();
    let base = format!("http://127.0.0.1:{port}");
    let page = client
        .get(format!("{base}/"))
        .send()
        .await
        .expect("production page");
    assert_eq!(page.status(), 200);
    let header = |name: &str| {
        page.headers()
            .get(name)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_owned()
    };

    // Secure headers, including a per-request CSP nonce.
    assert!(header("strict-transport-security").starts_with("max-age="));
    assert!(header("content-security-policy").contains("'nonce-"));
    assert_eq!(header("x-frame-options"), "DENY");
    assert_eq!(header("x-content-type-options"), "nosniff");
    // The double-submit CSRF cookie is issued with `Secure`.
    let cookie = header("set-cookie");
    assert!(cookie.contains("; Secure"), "production CSRF cookie");
    let token = cookie
        .split(';')
        .next()
        .and_then(|pair| pair.strip_prefix("rullst_csrf="))
        .expect("CSRF cookie")
        .to_owned();
    assert_eq!(page.text().await.expect("page body"), "OK");

    // CSRF: a write without the token is refused before the handler runs.
    let denied = client
        .post(format!("{base}/write"))
        .send()
        .await
        .expect("write without token");
    assert_eq!(denied.status(), 403);
    let accepted = client
        .post(format!("{base}/write"))
        .header("cookie", format!("rullst_csrf={token}"))
        .header("x-csrf-token", &token)
        .send()
        .await
        .expect("write with token");
    assert_eq!(accepted.status(), 200);
    assert_eq!(accepted.text().await.expect("write body"), "written");

    // WAF: a script payload in the query string is blocked.
    let blocked = client
        .get(format!("{base}/?q=%3Cscript%3E"))
        .send()
        .await
        .expect("WAF probe");
    assert_eq!(blocked.status(), 403);

    handle.abort();
}
