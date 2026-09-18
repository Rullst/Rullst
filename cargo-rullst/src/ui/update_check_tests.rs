use super::*;
use std::ffi::OsStr;
use std::io::Cursor;

fn catalog(versions: &[(&str, bool, &str)]) -> Vec<u8> {
    serde_json::to_vec(
        &serde_json::json!({"versions": versions.iter().map(|(num, yanked, package)| {
        serde_json::json!({"num": num, "yanked": yanked, "crate": package})
    }).collect::<Vec<_>>()}),
    )
    .unwrap()
}

#[test]
fn offline_and_ci_flag_values_are_explicit() {
    for value in ["true", "1", "YES", " True "] {
        assert!(enabled_env_flag(Some(OsStr::new(value))));
    }
    for value in ["false", "0", "no", "", "arbitrary"] {
        assert!(!enabled_env_flag(Some(OsStr::new(value))));
    }
    assert!(!enabled_env_flag(None));
}

#[test]
fn selects_highest_stable_same_major_not_the_catalog_order() {
    let bytes = catalog(&[
        ("12.0.1", false, "cargo-rullst"),
        ("13.0.0", false, "cargo-rullst"),
        ("12.2.0", false, "cargo-rullst"),
        ("12.1.0", false, "cargo-rullst"),
    ]);
    assert_eq!(
        select_update(Cursor::new(bytes), "12.0.0").unwrap(),
        Some(Version::parse("12.2.0").unwrap())
    );
}

#[test]
fn ignores_yanked_prerelease_foreign_invalid_and_build_metadata_versions() {
    let bytes = catalog(&[
        ("12.4.0", true, "cargo-rullst"),
        ("12.3.0-rc.1", false, "cargo-rullst"),
        ("12.5.0", false, "different-package"),
        ("12.6.0+build", false, "cargo-rullst"),
        ("12.7.0\u{1b}[31m", false, "cargo-rullst"),
        ("not-semver", false, "cargo-rullst"),
    ]);
    assert_eq!(select_update(Cursor::new(bytes), "12.0.0").unwrap(), None);
}

#[test]
fn stable_release_can_replace_its_rc_but_another_rc_is_not_selected() {
    let bytes = catalog(&[
        ("12.0.0", false, "cargo-rullst"),
        ("12.0.0-rc.2", false, "cargo-rullst"),
    ]);
    assert_eq!(
        select_update(Cursor::new(bytes), "12.0.0-rc.1").unwrap(),
        Some(Version::parse("12.0.0").unwrap())
    );
}

#[test]
fn equal_versions_and_downgrades_never_become_notices() {
    let bytes = catalog(&[
        ("12.0.0", false, "cargo-rullst"),
        ("11.99.0", false, "cargo-rullst"),
    ]);
    assert_eq!(select_update(Cursor::new(bytes), "12.0.0").unwrap(), None);
    assert_eq!(
        select_update(Cursor::new(catalog(&[])), "12.0.0").unwrap(),
        None
    );
}

#[test]
fn malformed_catalog_and_missing_yank_status_fail_closed() {
    for bytes in [
        b"not-json".as_slice(),
        br#"{"versions":[{"num":"12.1.0","crate":"cargo-rullst"}]}"#,
        br#"{"versions":[],"versions":[]}"#,
    ] {
        assert!(matches!(
            select_update(Cursor::new(bytes), "12.0.0"),
            Err(DiscoveryError::Json(_))
        ));
    }
    assert!(matches!(
        select_update(Cursor::new(catalog(&[])), "invalid"),
        Err(DiscoveryError::Version(_))
    ));
}

#[test]
fn response_limit_is_enforced_even_for_an_infinite_reader() {
    assert!(matches!(
        select_update(std::io::repeat(b' '), "12.0.0"),
        Err(DiscoveryError::TooLarge)
    ));
    let mut exact = catalog(&[]);
    exact.resize(CATALOG_LIMIT as usize, b' ');
    assert_eq!(select_update(Cursor::new(&exact), "12.0.0").unwrap(), None);
    exact.push(b' ');
    assert!(matches!(
        select_update(Cursor::new(exact), "12.0.0"),
        Err(DiscoveryError::TooLarge)
    ));
}

#[test]
fn read_failure_does_not_become_an_update() {
    struct BrokenReader;
    impl Read for BrokenReader {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("injected read failure"))
        }
    }
    assert!(matches!(
        select_update(BrokenReader, "12.0.0"),
        Err(DiscoveryError::Read(_))
    ));
}

#[test]
fn version_notice_rejects_other_trains_and_terminal_escapes() {
    let current = Version::parse("12.0.0").unwrap();
    for version in ["13.0.0", "12.1.0-rc.1", "12.1.0+dev", "11.9.0"] {
        assert!(!eligible_update(
            &current,
            &Version::parse(version).unwrap()
        ));
    }
    print_update_banner("12.1.0\u{1b}[2J");
    assert!(Version::parse("12.1.0\u{1b}[2J").is_err());
}

async fn server(
    response: Vec<u8>,
    trickle: bool,
) -> (
    String,
    tokio::task::JoinHandle<()>,
    std::sync::Arc<std::sync::atomic::AtomicUsize>,
) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/catalog", listener.local_addr().unwrap());
    let count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let sent = count.clone();
    let task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = [0; 4096];
        let mut received = 0;
        while !request[..received]
            .windows(4)
            .any(|bytes| bytes == b"\r\n\r\n")
        {
            assert!(
                received < request.len(),
                "fixture request headers exceed limit"
            );
            let size = stream.read(&mut request[received..]).await.unwrap();
            assert!(size > 0, "fixture request closed before headers finished");
            received += size;
        }
        stream.write_all(&response).await.unwrap();
        if trickle {
            for _ in 0..100 {
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                if stream.write_all(b" ").await.is_err() {
                    break;
                }
                sent.fetch_add(1, Ordering::Relaxed);
            }
        }
    });
    (url, task, count)
}

fn response(status: &str, body: &[u8]) -> Vec<u8> {
    let mut bytes = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    bytes.extend_from_slice(body);
    bytes
}

fn local_client() -> reqwest::Client {
    // The private loopback fixture is the only place HTTPS is disabled.
    discovery_client()
        .https_only(false)
        .no_proxy()
        .build()
        .unwrap()
}

#[tokio::test]
async fn real_http_catalog_selects_the_allowed_update() {
    let bytes = catalog(&[("12.1.0", false, "cargo-rullst")]);
    let (url, task, _) = server(response("200 OK", &bytes), false).await;
    let found = bounded_discovery(
        &local_client(),
        &url,
        "12.0.0",
        std::time::Duration::from_secs(2),
    )
    .await
    .unwrap();
    assert_eq!(found, Some(Version::parse("12.1.0").unwrap()));
    task.await.unwrap();
}

#[tokio::test]
async fn redirects_are_not_followed_and_http_errors_are_not_catalogs() {
    let (url, task, _) = server(b"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/must-not-connect\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(), false).await;
    assert_eq!(
        bounded_discovery(
            &local_client(),
            &url,
            "12.0.0",
            std::time::Duration::from_secs(2)
        )
        .await
        .unwrap(),
        None
    );
    task.await.unwrap();
    let (url, task, _) = server(
        response(
            "503 Service Unavailable",
            &catalog(&[("12.1.0", false, "cargo-rullst")]),
        ),
        false,
    )
    .await;
    assert!(matches!(
        bounded_discovery(
            &local_client(),
            &url,
            "12.0.0",
            std::time::Duration::from_secs(2)
        )
        .await,
        Err(DiscoveryError::Http(_))
    ));
    task.await.unwrap();
}

#[tokio::test]
async fn total_deadline_rejects_a_continuously_trickling_response() {
    let (url, task, count) = server(
        b"HTTP/1.1 200 OK\r\nContent-Length: 100000\r\nConnection: close\r\n\r\n".to_vec(),
        true,
    )
    .await;
    let result = bounded_discovery(
        &local_client(),
        &url,
        "12.0.0",
        std::time::Duration::from_millis(500),
    )
    .await;
    task.abort();
    let _ = task.await;
    assert!(
        count.load(Ordering::Relaxed) >= 2,
        "the server must actually trickle body chunks before the deadline"
    );
    assert!(matches!(result, Err(DiscoveryError::Timeout)));
}

#[tokio::test]
async fn production_client_refuses_plain_http() {
    let client = discovery_client().no_proxy().build().unwrap();
    let result = bounded_discovery(
        &client,
        "http://127.0.0.1:1/not-https",
        "12.0.0",
        std::time::Duration::from_secs(2),
    )
    .await;
    assert!(matches!(result, Err(DiscoveryError::Http(_))));
}

#[tokio::test]
async fn streamed_oversize_and_empty_responses_fail_closed() {
    let (url, task, _) = server(
        response("200 OK", &vec![b' '; CATALOG_LIMIT as usize + 1]),
        false,
    )
    .await;
    let result = bounded_discovery(
        &local_client(),
        &url,
        "12.0.0",
        std::time::Duration::from_secs(2),
    )
    .await;
    task.await.unwrap();
    assert!(matches!(result, Err(DiscoveryError::TooLarge)));
    let (url, task, _) = server(response("204 No Content", &[]), false).await;
    assert_eq!(
        bounded_discovery(
            &local_client(),
            &url,
            "12.0.0",
            std::time::Duration::from_secs(2)
        )
        .await
        .unwrap(),
        None
    );
    task.await.unwrap();
}
