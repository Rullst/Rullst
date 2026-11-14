//! `--monitor-url`: URL checks, route matching, reading a monitor document
//! from a loopback server and the report section.
use super::super::production::{self, MONITOR_PATH, Production, validate_monitor_url};
use super::run::footprint_with;
use super::support::{Request, TestServer, response};
use super::*;
use std::sync::Arc;

const TOKEN: &str = "DjxqcYY6VXbaPLLGGkjMN3aJCjqkMo8TcEjFfXD6f7Y=";

fn document(bytes: bool) -> serde_json::Value {
    let route = |method: &str, route: &str, requests: u64, sent: u64| {
        let mut value = serde_json::json!({
            "method": method,
            "route": route,
            "requests_total": requests,
            "status_total": {"1xx": 0, "2xx": requests, "3xx": 0, "4xx": 0, "5xx": 0},
            "window": {"covered_seconds": 300, "requests": requests / 10, "bytes": sent / 10},
        });
        if !bytes {
            value["window"].as_object_mut().unwrap().remove("bytes");
        } else {
            value["bytes_total"] = sent.into();
        }
        value
    };
    let mut http = serde_json::json!({
        "window_seconds": 300,
        "requests_total": 1_300,
        "routes": [
            route("GET", "/", 1_000, 12_288_000),
            route("GET", "/posts/{id}", 200, 1_024_000),
            route("GET", "/posts/new", 50, 10_240),
            route("POST", "/posts", 50, 5_000),
            route("GET", "/static/{*rest}", 0, 0),
        ],
    });
    if bytes {
        http["bytes_total"] = 13_327_240.into();
    }
    serde_json::json!({
        "schema": "rullst.monitor.v1",
        "application": {"name": "shop\u{1b}[31m", "version": "1.4.0"},
        "uptime_seconds": 86_400,
        "http": http,
    })
}

#[test]
fn only_https_or_loopback_monitor_urls_are_accepted() {
    for (raw, expected) in [
        (
            "https://app.example",
            "https://app.example/_rullst/monitor/v1",
        ),
        (
            "https://app.example/_rullst/monitor/v1",
            "https://app.example/_rullst/monitor/v1",
        ),
        (
            "https://app.example/shop/_rullst/monitor/v1",
            "https://app.example/shop/_rullst/monitor/v1",
        ),
        (
            "http://127.0.0.1:9000",
            "http://127.0.0.1:9000/_rullst/monitor/v1",
        ),
        (
            "http://localhost:9000/",
            "http://localhost:9000/_rullst/monitor/v1",
        ),
        ("http://[::1]:9000", "http://[::1]:9000/_rullst/monitor/v1"),
    ] {
        assert_eq!(
            validate_monitor_url(raw).as_ref().map(reqwest::Url::as_str),
            Ok(expected),
            "{raw}"
        );
    }
    for (raw, error) in [
        ("http://app.example", "must use https://"),
        ("http://10.0.0.1:9000", "must use https://"),
        ("ftp://app.example", "must use https://"),
        (
            "https://user:secret@app.example",
            "must not contain credentials",
        ),
        (
            "https://app.example/?token=x",
            "must not contain credentials",
        ),
        ("https://app.example/#x", "must not contain credentials"),
        (
            "https://app.example/health",
            "ending in /_rullst/monitor/v1",
        ),
        ("not a url", "is not a valid URL"),
    ] {
        let refused = validate_monitor_url(raw).unwrap_err();
        assert!(refused.contains(error), "{raw}: {refused}");
    }
    // The token is never a command-line argument.
    assert!(
        options::command()
            .try_get_matches_from(["footprint", "--monitor-token", TOKEN])
            .is_err()
    );
}

#[test]
fn route_patterns_match_their_paths() {
    for (pattern, path) in [
        ("/", "/"),
        ("/posts/{id}", "/posts/7"),
        ("/posts/{id}", "/posts/7?draft=1"),
        ("/static/{*rest}", "/static/css/app.css"),
        ("/static/{*rest}", "/static/"),
        ("/a/{x}/b", "/a/1/b"),
    ] {
        assert!(production::route_matches(pattern, path), "{pattern} {path}");
    }
    for (pattern, path) in [
        ("/", "/posts"),
        ("/posts/{id}", "/posts"),
        ("/posts/{id}", "/posts/"),
        ("/posts/{id}", "/posts/7/edit"),
        ("/posts", "/posts/7"),
        ("/a/{x}/b", "/a/1/c"),
    ] {
        assert!(
            !production::route_matches(pattern, path),
            "{pattern} {path}"
        );
    }
}

#[test]
fn a_monitor_document_gives_bytes_per_route_and_the_page_route() {
    let found = production::from_document(
        &document(true),
        "https://app.example/_rullst/monitor/v1",
        "/posts/new",
    );
    assert_eq!(found.status, "measured");
    assert_eq!(found.application.as_deref(), Some("shop\\u{1b}[31m 1.4.0"));
    assert_eq!(found.uptime_seconds, Some(86_400));
    assert_eq!(found.window_seconds, Some(300));
    assert_eq!(found.bytes_reported, Some(true));
    assert_eq!(
        (found.requests_total, found.bytes_total),
        (Some(1_300), Some(13_327_240))
    );
    assert_eq!(found.routes.len(), 5);
    // The literal route wins over the pattern that also matches.
    let page = found.path_route.clone().unwrap();
    assert_eq!(
        (page.method.as_str(), page.route.as_str()),
        ("GET", "/posts/new")
    );
    assert_eq!(page.bytes_per_request, Some(204.8));
    assert_eq!(page.window_bytes, Some(1_024));
    assert_eq!(found.routes[4].bytes_per_request, None);
    let instance = production::from_document(&document(true), "x", "/posts/9");
    assert_eq!(instance.path_route.unwrap().route, "/posts/{id}");
    assert_eq!(
        production::from_document(&document(true), "x", "/missing").path_route,
        None
    );

    let older = production::from_document(&document(false), "x", "/");
    assert_eq!(older.bytes_reported, Some(false));
    assert_eq!(older.routes[0].bytes_total, None);
    assert_eq!(older.routes[0].window_bytes, None);

    let other = production::from_document(&serde_json::json!({"schema": "x"}), "x", "/");
    assert_eq!(other.status, "not_measured");
    assert_eq!(
        other.reason.as_deref(),
        Some("the monitor did not answer with a rullst.monitor.v1 document")
    );
}

fn monitor_server() -> TestServer {
    let body = serde_json::to_vec(&document(true)).unwrap();
    TestServer::with(Arc::new(move |request: &Request| {
        let expected = format!("Bearer {TOKEN}");
        let authorized = request.header("authorization") == Some(expected.as_str());
        match (request.path.as_str(), authorized) {
            (MONITOR_PATH, true) => response("200 OK", "content-type: application/json\r\n", &body),
            (MONITOR_PATH, false) => response("401 Unauthorized", "", b""),
            ("/text", _) => response("200 OK", "", b"not json"),
            _ => response("404 Not Found", "", b""),
        }
    }))
}

fn read(server: &TestServer, path: &str, token: Option<&str>) -> Production {
    let url = server.url(path);
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(production::read(&url, token, "/"))
}

#[test]
fn the_monitor_is_read_with_the_token_and_failures_are_explained() {
    let server = monitor_server();
    let found = read(&server, MONITOR_PATH, Some(TOKEN));
    assert_eq!(found.status, "measured", "{found:?}");
    assert_eq!(
        found.source.as_deref(),
        Some(server.url(MONITOR_PATH).as_str())
    );
    assert_eq!(found.path_route.unwrap().route, "/");

    let no_token = read(&server, MONITOR_PATH, Some("  "));
    assert_eq!(no_token.status, "not_measured");
    assert!(
        no_token
            .reason
            .unwrap()
            .starts_with("set RULLST_MONITOR_TOKEN")
    );
    assert_eq!(
        server.paths().len(),
        1,
        "no request is sent without a token"
    );

    let wrong = read(&server, MONITOR_PATH, Some("wrong"));
    assert_eq!(
        wrong.reason.as_deref(),
        Some("the monitor answered 401; check RULLST_MONITOR_TOKEN")
    );
    let missing = read(&server, "/elsewhere", Some(TOKEN));
    assert!(
        missing
            .reason
            .unwrap()
            .starts_with("the monitor answered 404; the application does not mount the monitor")
    );
    let text = read(&server, "/text", Some(TOKEN));
    assert_eq!(
        text.reason.as_deref(),
        Some("the monitor's answer is not JSON")
    );
}

#[test]
fn the_production_section_sits_next_to_the_lab_figures() {
    let plain = Style { color: false };
    assert!(production::render(&Production::unmeasured("not_run", "x"), None, plain).is_empty());
    let found = production::from_document(
        &document(true),
        "https://app.example/_rullst/monitor/v1",
        "/",
    );
    let text = production::render(&found, Some(11_000.0), plain);
    for expected in [
        "Production (monitor)",
        "  Source        shop\\u{1b}[31m 1.4.0  https://app.example/_rullst/monitor/v1, up 86400 s",
        "  Served        12.7 MiB  1300 requests since start, bodies as sent after compression",
        "  This page     12.0 KiB/request  GET / in production; lab: 10.7 KiB/request",
        "  Route         11.7 MiB  GET /: 1000 requests, 12.0 KiB/request",
        "  Route         0 B  GET /static/{*rest}: 0 requests, -",
    ] {
        assert!(text.contains(expected), "missing `{expected}` in:\n{text}");
    }
    assert!(!text.contains('\u{1b}'), "{text}");
    let unlabelled = production::render(&found, None, plain);
    assert!(
        unlabelled.contains("lab: pass --bench to compare"),
        "{unlabelled}"
    );
    let older = production::from_document(&document(false), "x", "/");
    assert!(production::render(&older, None, plain).contains("Bytes         NOT REPORTED"));
    let failed = Production::unmeasured("not_measured", "the monitor answered 401");
    assert!(
        production::render(&failed, None, plain).contains("NOT MEASURED  the monitor answered 401")
    );
}

/// The token reaches the child only through its environment; the report
/// keeps the URL and the counters, never the token.
#[test]
fn the_run_reads_the_monitor_with_the_token_from_the_environment() {
    let server = monitor_server();
    let monitor = server.base.to_string();
    let (_, outcome, shown, stderr) = footprint_with(
        &[],
        &[("RULLST_MONITOR_TOKEN", TOKEN)],
        None,
        &["--monitor-url", monitor.as_str(), "--json"],
    );
    assert_eq!(outcome, "ok");
    assert!(!shown.contains(TOKEN));
    assert!(!stderr.contains(TOKEN));
    let report: serde_json::Value = serde_json::from_str(&shown).unwrap();
    assert_eq!(report["production"]["status"], "measured");
    assert_eq!(report["production"]["path_route"]["route"], "/");
    assert_eq!(report["production"]["bytes_total"], 13_327_240);
    assert!(
        !stderr.contains("Reading the production monitor"),
        "{stderr}"
    );

    let (_, outcome, shown, stderr) =
        footprint_with(&[], &[], None, &["--monitor-url", monitor.as_str()]);
    assert_eq!(outcome, "ok");
    assert!(
        stderr.contains("Reading the production monitor at"),
        "{stderr}"
    );
    assert!(
        shown.contains("NOT MEASURED  set RULLST_MONITOR_TOKEN"),
        "{shown}"
    );
}
