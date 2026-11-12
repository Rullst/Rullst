//! `footprint --url` against a tiny in-process axum server: the real benchmark
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
    std::fs::write(
        directory.path().join("Cargo.lock"),
        "version = 4\n\n[[package]]\nname = \"app\"\nversion = \"0.1.0\"\n\n[[package]]\nname = \"serde\"\nversion = \"1.0.0\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n",
    )
    .unwrap();
    Command::new(env!("CARGO_BIN_EXE_cargo-rullst"))
        .args(["rullst", "footprint"])
        .args(arguments)
        .current_dir(directory.path())
        .env("NO_COLOR", "1")
        .env("RULLST_DISABLE_UPDATE_CHECK", "1")
        .env("DOCKER_HOST", "unix:///nonexistent")
        .env("CONTAINER_HOST", "unix:///nonexistent")
        .output()
        .unwrap()
}

#[test]
fn the_benchmark_measures_cpu_and_labels_the_estimate() {
    let port = serve();
    let url = format!("http://127.0.0.1:{port}");
    let output = footprint(&[
        "--url",
        &url,
        "--image",
        "rullst-footprint-test-missing:latest",
        "--bench",
        "--duration",
        "1s",
        "--concurrency",
        "2",
        "--cpu-watts",
        "10",
        "--grid-intensity",
        "100",
        "--network-kwh-per-gb",
        "0.059",
        "--page-weight",
        "--json",
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["schema_version"], "rullst.footprint.v1");
    assert_eq!(report["schema"], "rullst.footprint.v1");
    assert_eq!(report["target"]["mode"], "existing_url");
    assert_eq!(report["target"]["url"], format!("{url}/"));
    let benchmark = &report["benchmark"];
    assert_eq!(benchmark["status"], "run");
    assert!(benchmark["requests"].as_u64().unwrap() > 0, "{benchmark}");
    assert_eq!(benchmark["transport_errors"], 0, "{benchmark}");
    assert_eq!(benchmark["http_errors"], 0, "{benchmark}");
    assert_eq!(report["measured"]["dependency_count"]["value"], 1);
    assert_eq!(
        report["measured"]["image_size_bytes"]["status"],
        "not_measured"
    );
    assert_eq!(report["sci_estimate"]["label"], "estimate");
    assert_eq!(report["sci_estimate"]["inputs"]["cpu_watts"], 10.0);
    assert_eq!(
        report["sci_estimate"]["inputs"]["grid_intensity_g_per_kwh"],
        100.0
    );
    assert_eq!(report["page_weight"]["status"], "measured", "{report}");
    assert_eq!(report["page_weight"]["first_visit"]["requests"], 1);
    assert_eq!(report["page_weight"]["first_visit"]["bytes"], 2);
    let requests = benchmark["requests"].as_u64().unwrap();
    assert_eq!(
        benchmark["response_bytes"]["value"].as_u64(),
        Some(2 * requests)
    );
    assert_eq!(
        report["sci_estimate"]["inputs"]["network_kwh_per_gb"],
        0.059
    );
    if cfg!(any(target_os = "linux", target_os = "macos", windows)) {
        // This test process owns the listening socket; it is found by its
        // port and sampled on every supported platform.
        assert_eq!(report["target"]["pid"], std::process::id());
        assert_eq!(benchmark["cpu_time_seconds"]["status"], "measured");
        assert_eq!(report["measured"]["idle_rss_bytes"]["status"], "measured");
        assert_eq!(
            report["measured"]["binary_size_bytes"]["status"],
            "measured"
        );
        assert_eq!(report["sci_estimate"]["status"], "estimate");
        assert!(
            report["sci_estimate"]["sci_g_per_request"]
                .as_f64()
                .is_some()
        );
    }

    let text = footprint(&["--url", &url]);
    assert!(text.status.success());
    let text = String::from_utf8_lossy(&text.stdout);
    assert!(text.contains("Benchmark     NOT RUN"), "{text}");
    assert!(text.contains("Page weight   NOT RUN"), "{text}");
    assert!(
        text.contains("not a certification, an environmental claim"),
        "{text}"
    );
    assert!(text.contains("SCI estimate (not a measurement)"), "{text}");
    assert!(!text.contains('\u{1b}'), "NO_COLOR output is plain");
}

#[test]
fn non_loopback_and_unreachable_targets_fail() {
    let refused = footprint(&["--url", "http://example.com", "--json"]);
    assert_eq!(refused.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&refused.stderr).contains("loopback"));
    assert!(refused.stdout.is_empty());

    // A parallel test's server may bind the freed port before `footprint` runs,
    // which then benchmarks it; try a few fresh ports until one is refused.
    let unreachable = (0..5)
        .map(|_| {
            let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let port = closed.local_addr().unwrap().port();
            drop(closed);
            footprint(&["--url", &format!("http://127.0.0.1:{port}"), "--json"])
        })
        .find(|output| !output.status.success())
        .expect("every freed port was answered");
    assert_eq!(unreachable.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&unreachable.stderr).contains("App not reachable"));
    // The JSON report still carries the figures that need no running app,
    // with the failure inside.
    let report: Value =
        serde_json::from_slice(&unreachable.stdout).expect("stdout holds only the JSON report");
    assert_eq!(report["schema"], "rullst.footprint.v1");
    assert_eq!(report["failure"]["stage"], "reach");
    assert_eq!(
        report["measured"]["idle_rss_bytes"]["reason"],
        "the app was not reachable"
    );

    let estimate_without_bench = footprint(&["--cpu-watts", "10"]);
    assert_eq!(estimate_without_bench.status.code(), Some(2));
}

#[test]
fn green_is_not_a_command_any_more() {
    let green = Command::new(env!("CARGO_BIN_EXE_cargo-rullst"))
        .args(["rullst", "green", "--json"])
        .env("NO_COLOR", "1")
        .env("RULLST_DISABLE_UPDATE_CHECK", "1")
        .output()
        .unwrap();
    assert_eq!(green.status.code(), Some(2));
    assert!(green.stdout.is_empty());
    let help = Command::new(env!("CARGO_BIN_EXE_cargo-rullst"))
        .args(["rullst", "footprint", "--help"])
        .env("NO_COLOR", "1")
        .env("RULLST_DISABLE_UPDATE_CHECK", "1")
        .output()
        .unwrap();
    assert!(help.status.success());
    let help = String::from_utf8_lossy(&help.stdout);
    assert!(
        help.contains("not a certification or an environmental claim"),
        "{help}"
    );
    assert!(help.contains("--page-weight"), "{help}");
}
