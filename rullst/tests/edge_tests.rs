use rullst::db::{ReplicationConfig, ReplicationError, ReplicationManager};
use rullst::edge::{EdgeRequest, EdgeResponse, EdgeServer};
use std::collections::HashMap;

fn free_loopback_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .expect("an ephemeral loopback port should be available")
        .local_addr()
        .expect("the loopback listener should have a local address")
        .port()
}

#[test]
fn test_edge_request_builder() {
    let req = EdgeRequest::new("POST", "/submit")
        .with_header("Content-Type", "application/json")
        .with_body(b"{\"hello\": \"world\"}".to_vec());

    assert_eq!(req.method, "POST");
    assert_eq!(req.path, "/submit");
    assert_eq!(
        req.headers.get("Content-Type"),
        Some(&"application/json".to_string())
    );
    assert_eq!(req.body, b"{\"hello\": \"world\"}".to_vec());
}

#[test]
fn test_edge_response_builder() {
    let resp = EdgeResponse::new(200)
        .with_header("Content-Type", "text/plain")
        .with_body(b"OK".to_vec());

    assert_eq!(resp.status, 200);
    assert_eq!(
        resp.headers.get("Content-Type"),
        Some(&"text/plain".to_string())
    );
    assert_eq!(resp.body, b"OK".to_vec());
}

#[tokio::test]
async fn test_edge_spawner() {
    let (tx, rx) = tokio::sync::oneshot::channel::<bool>();

    rullst::edge::spawn(async move {
        let _ = tx.send(true);
    });

    let val = rx.await.unwrap();
    assert!(val);
}

#[tokio::test]
async fn test_edge_server_emulation_handler() {
    // We construct a mock edge server with a working handler and execute its inner logic emulated
    let handler = |req: EdgeRequest| async move {
        let mut headers = HashMap::new();
        headers.insert("X-Edge-Header".to_string(), "active".to_string());

        let mut body_str = String::from_utf8_lossy(&req.body).into_owned();
        body_str.push_str(" - EdgeProcessed");

        EdgeResponse::new(201)
            .with_header("X-Edge-Header", "active")
            .with_body(body_str.into_bytes())
    };

    let server = EdgeServer::new(handler).with_port(9999);
    assert_eq!(server.port, 9999);

    // Perform emulated execution directly
    let req = EdgeRequest::new("GET", "/test").with_body(b"Input".to_vec());
    let resp = (server.handler)(req).await;

    assert_eq!(resp.status, 201);
    assert_eq!(
        resp.headers.get("X-Edge-Header"),
        Some(&"active".to_string())
    );
    assert_eq!(resp.body, b"Input - EdgeProcessed".to_vec());
}

#[test]
fn test_replication_config_builder() {
    let config = ReplicationConfig::new("local.db")
        .with_sync_url("libsql://replica.turso.io")
        .with_auth_token("super-secret-token")
        .with_sync_interval(5);

    assert_eq!(config.replica_path, "local.db");
    assert_eq!(
        config.sync_url,
        Some("libsql://replica.turso.io".to_string())
    );
    assert_eq!(config.auth_token, Some("super-secret-token".to_string()));
    assert_eq!(config.sync_interval_secs, 5);
}

#[test]
fn test_replication_manager_rejects_unimplemented_backend() {
    let config = ReplicationConfig::new("local.db")
        .with_sync_url("libsql://replica.turso.io")
        .with_auth_token("token")
        .with_sync_interval(1);

    assert!(matches!(
        ReplicationManager::start(config),
        Err(ReplicationError::Unsupported { .. })
    ));
}

#[tokio::test]
async fn test_edge_module_exists() {}

#[tokio::test]
#[cfg(not(miri))]
async fn test_edge_server_run_integration() {
    let handler = |_req: EdgeRequest| async move {
        EdgeResponse::new(200)
            .with_header("X-Edge-Test", "Passed")
            .with_body(b"Hello from EdgeServer!".to_vec())
    };

    let port = free_loopback_port();
    let server = EdgeServer::new(handler).with_port(port);

    // Spawn the server in the background
    let handle = tokio::spawn(async move { server.run().await.map_err(|error| error.to_string()) });

    // Retry connecting with backoff for robustness under heavy CI / test concurrency
    let client = reqwest::Client::new();
    let mut last_res = None;
    for _ in 0..50 {
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        if handle.is_finished() {
            break;
        }
        if let Ok(res) = client
            .get(format!("http://127.0.0.1:{}/test/integration", port))
            .send()
            .await
        {
            last_res = Some(res);
            break;
        }
    }

    let Some(res) = last_res else {
        if handle.is_finished() {
            let server_result = handle
                .await
                .expect("EdgeServer task should not panic before accepting requests");
            panic!("EdgeServer stopped before accepting requests: {server_result:?}");
        }
        handle.abort();
        panic!("Failed to execute request to EdgeServer after retry window");
    };

    assert_eq!(res.status().as_u16(), 200);
    assert_eq!(
        res.headers().get("X-Edge-Test").unwrap().to_str().unwrap(),
        "Passed"
    );
    let body = res.text().await.unwrap();
    assert_eq!(body, "Hello from EdgeServer!");

    handle.abort();
}
