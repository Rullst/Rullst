//! `footprint --url` against a tiny in-process axum server: the real load
//! loop and report, with no project build and no network beyond loopback.
use serde_json::Value;
use std::process::{Command, Output};

/// Serves `/` on a free loopback port from a background runtime.
fn serve() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async move {
            let listener = tokio::net::TcpListener::from_std(listener).unwrap();
            let app = axum::Router::new().route("/", axum::routing::get(|| async { "ok" }));
            axum::serve(listener, app).await.unwrap();
        });
    });
    port
}

fn footprint(arguments: &[&str]) -> Output {
    let directory = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_rullst"))
        .arg("footprint")
        .args(arguments)
        .current_dir(directory.path())
        .env("NO_COLOR", "1")
        .env("DOCKER_HOST", "unix:///nonexistent")
        .output()
        .unwrap()
}

#[test]
fn the_load_loop_populates_throughput_latency_and_labels() {
    let port = serve();
    let url = format!("http://127.0.0.1:{port}");
    let output = footprint(&[
        "--url",
        &url,
        "--duration",
        "1s",
        "--concurrency",
        "2",
        "--cpu-watts",
        "10",
        "--grid-intensity",
        "100",
        "--json",
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["schema_version"], "rullst.cli-footprint.v1");
    assert_eq!(report["target"]["mode"], "existing_url");
    assert_eq!(report["inputs"]["url"], format!("{url}/"));
    let load = &report["load"];
    assert!(load["requests"].as_u64().unwrap() > 0, "{load}");
    assert_eq!(load["errors"], 0, "{load}");
    assert!(load["requests_per_second"].as_f64().unwrap() > 0.0);
    for percentile in ["p50", "p95", "p99"] {
        assert!(
            load["latency_ms"][percentile].as_f64().unwrap() > 0.0,
            "{percentile}"
        );
    }
    assert!(load["latency_ms"]["p50"].as_f64() <= load["latency_ms"]["p99"].as_f64());
    assert_eq!(
        report["artifacts"]["docker_image_size_bytes"]["status"],
        "not_measured"
    );
    if cfg!(target_os = "linux") {
        // This test process owns the listening socket.
        assert_eq!(report["target"]["pid"], std::process::id());
        assert_eq!(report["process"]["cpu_time_seconds"]["status"], "measured");
        assert_eq!(report["process"]["peak_rss_bytes"]["status"], "measured");
        assert_eq!(
            report["artifacts"]["binary_size_bytes"]["status"],
            "measured"
        );
        // RAPL when readable (root), otherwise the labelled CPU-time estimate.
        let energy = report["energy"]["status"].as_str().unwrap();
        assert!(["measured", "estimate"].contains(&energy), "{energy}");
        assert_eq!(report["carbon"]["status"], "computed");
        assert_eq!(
            report["carbon"]["i_source"],
            "user-provided (--grid-intensity)"
        );
        assert_eq!(report["carbon"]["m_source"], "not included");
        if energy == "estimate" {
            assert_eq!(report["carbon"]["estimated_terms"][0], "E");
        }
    }

    let text = footprint(&["--url", &url, "--duration", "1s"]);
    assert!(text.status.success());
    let text = String::from_utf8_lossy(&text.stdout);
    assert!(text.contains("Requests/s"), "{text}");
    assert!(text.contains("Latency p99"), "{text}");
    assert!(!text.contains('\u{1b}'), "NO_COLOR output is plain");
}

#[test]
fn non_loopback_and_unreachable_targets_fail() {
    let refused = footprint(&["--url", "http://example.com", "--json"]);
    assert_eq!(refused.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&refused.stderr).contains("loopback"));
    assert!(refused.stdout.is_empty());

    let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = closed.local_addr().unwrap().port();
    drop(closed);
    let unreachable = footprint(&["--url", &format!("http://127.0.0.1:{port}"), "--json"]);
    assert_eq!(unreachable.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&unreachable.stderr).contains("App not reachable"));
    assert!(unreachable.stdout.is_empty());
}
