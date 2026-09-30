//! Unit tests for error console parsing and source context extraction.

#![allow(clippy::unwrap_used)]

use super::*;

#[test]
fn test_find_source_location_linux() {
    let bt = "   0: rullst::error_console::tests::test_panic\n             at /home/user/project/src/error_console.rs:42";
    let res = find_source_location(bt);
    assert_eq!(
        res,
        Some(("/home/user/project/src/error_console.rs".to_string(), 42))
    );
}

#[test]
fn test_find_source_location_windows() {
    let bt = "   0: rullst::error_console::tests::test_panic\n             at C:\\Users\\user\\project\\src\\error_console.rs:55";
    let res = find_source_location(bt);
    assert_eq!(
        res,
        Some((
            "C:\\Users\\user\\project\\src\\error_console.rs".to_string(),
            55
        ))
    );
}

#[test]
fn test_find_source_location_none() {
    let bt = "   0: rust_panic\n             at /home/user/project/main.rs:100";
    let res = find_source_location(bt);
    assert_eq!(res, None);
}

#[test]
fn test_extract_source_context_bounds() {
    use std::io::Write;
    let cwd = std::env::current_dir().unwrap();
    let test_file = cwd.join("test_extract_source_context.rs");
    let mut file = std::fs::File::create(&test_file).unwrap();
    writeln!(file, "line 1").unwrap();
    writeln!(file, "line 2").unwrap();
    writeln!(file, "line 3").unwrap();
    file.sync_all().unwrap();

    let path_str = test_file.to_str().unwrap();

    // Testing line 1 (boundary)
    let ctx = extract_source_context(path_str, 1, 1).unwrap();
    assert_eq!(ctx.len(), 2);
    assert_eq!(ctx[0].1, "line 1");
    assert!(ctx[0].2); // is_target

    // Testing end of file
    let ctx = extract_source_context(path_str, 3, 1).unwrap();
    assert_eq!(ctx.len(), 2);
    assert_eq!(ctx[1].1, "line 3");
    assert!(ctx[1].2);

    let _ = std::fs::remove_file(test_file);
}

async fn panic_console_response(peer: Option<std::net::SocketAddr>) -> (u16, String) {
    use tower::ServiceExt;

    async fn panics() {
        panic!("secret panic payload");
    }

    let mut router = axum::Router::new()
        .route("/panic", axum::routing::get(panics))
        .layer(axum::middleware::from_fn(catch_panic_middleware));
    if let Some(peer) = peer {
        router = router.layer(axum::extract::connect_info::MockConnectInfo(peer));
    }
    let response = router
        .oneshot(
            axum::http::Request::get("/panic")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    (status, String::from_utf8_lossy(&body).into_owned())
}

#[tokio::test]
async fn panic_console_hides_details_from_non_loopback_peers() {
    let (status, body) =
        panic_console_response(Some(std::net::SocketAddr::from(([192, 168, 1, 7], 4000)))).await;
    assert_eq!(status, 500);
    assert!(!body.contains("secret panic payload"), "{body}");

    let mapped: std::net::SocketAddr = "[::ffff:10.0.0.1]:4000".parse().unwrap();
    let (status, body) = panic_console_response(Some(mapped)).await;
    assert_eq!(status, 500);
    assert!(!body.contains("secret panic payload"), "{body}");

    for peer in ["127.0.0.1:4000", "[::1]:4000", "[::ffff:127.0.0.1]:4000"] {
        let (status, body) = panic_console_response(Some(peer.parse().unwrap())).await;
        assert_eq!(status, 500);
        assert!(body.contains("secret panic payload"), "{peer}: {body}");
    }

    let (_, body) = panic_console_response(None).await;
    assert!(body.contains("secret panic payload"), "{body}");
}
