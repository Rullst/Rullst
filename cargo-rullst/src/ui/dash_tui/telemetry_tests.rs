#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::telemetry::{
    DatabaseReport, MAX_BODY_BYTES, MAX_REQUESTS, MAX_SLOW_QUERIES, PollOutcome, QueueReport,
    TELEMETRY_PATH, clean, parse_payload, poll,
};
use serde_json::{Value, json};

const GENERATION: &str = "0123456789abcdef0123456789abcdef";

fn document() -> Value {
    json!({
        "schema": "rullst.dev-telemetry.v1",
        "generation": GENERATION,
        "uptime_ms": 5_000,
        "http": {
            "requests_total": 3,
            "client_errors_total": 1,
            "server_errors_total": 1,
            "recent": [
                {"seq": 1, "method": "GET", "path": "/", "status": 200, "duration_us": 900},
                {"seq": 2, "method": "GET", "path": "/missing", "status": 404, "duration_us": 300},
                {"seq": 3, "method": "POST", "path": "/orders", "status": 500, "duration_us": 12_000}
            ]
        },
        "database": {
            "state": "observed",
            "queries_total": 9,
            "slow_queries_total": 1,
            "slow_threshold_ms": 100,
            "recent_slow": [
                {"seq": 1, "operation": "select_many", "model": "Post", "table": "posts", "duration_us": 150_000}
            ]
        },
        "queue": {"state": "observed", "pending": 4},
        "future_field": {"ignored": true}
    })
}

fn parse(value: &Value) -> Result<super::telemetry::TelemetrySnapshot, &'static str> {
    parse_payload(&serde_json::to_vec(value).unwrap())
}

#[test]
fn a_valid_document_is_parsed_and_unknown_fields_are_ignored() {
    let snapshot = parse(&document()).unwrap();
    assert_eq!(snapshot.generation, GENERATION);
    assert_eq!(snapshot.http.requests_total, 3);
    assert_eq!(snapshot.http.recent.len(), 3);
    assert_eq!(snapshot.http.recent[2].path, "/orders");
    assert_eq!(snapshot.http.recent[2].status, 500);
    assert_eq!(snapshot.queue, QueueReport::Observed { pending: 4 });
    let DatabaseReport::Observed {
        queries_total,
        slow_total,
        slow_threshold_ms,
        recent_slow,
    } = snapshot.database
    else {
        panic!("database should be observed");
    };
    assert_eq!((queries_total, slow_total, slow_threshold_ms), (9, 1, 100));
    assert_eq!(recent_slow[0].model.as_deref(), Some("Post"));
}

#[test]
fn malformed_or_inconsistent_documents_are_rejected() {
    let mutate = |change: &dyn Fn(&mut Value)| {
        let mut value = document();
        change(&mut value);
        parse(&value)
    };
    assert_eq!(parse_payload(b"not json"), Err("malformed JSON"));
    assert_eq!(parse_payload(b""), Err("malformed JSON"));
    assert_eq!(parse_payload(b"[]"), Err("malformed JSON"));
    assert_eq!(
        mutate(&|value| value["schema"] = json!("rullst.dev-telemetry.v2")),
        Err("unsupported schema")
    );
    for generation in [
        json!("ABCDEF0123456789abcdef0123456789"),
        json!("short"),
        json!(7),
    ] {
        let result = mutate(&|value| value["generation"] = generation.clone());
        assert!(result.is_err(), "{generation}");
    }
    assert_eq!(
        mutate(&|value| value["http"]["server_errors_total"] = json!(3)),
        Err("inconsistent counters")
    );
    assert_eq!(
        mutate(&|value| {
            value["http"]["client_errors_total"] = json!(u64::MAX);
            value["http"]["server_errors_total"] = json!(u64::MAX);
        }),
        Err("inconsistent counters")
    );
    for (field, bad) in [
        ("requests_total", json!(-1)),
        ("requests_total", json!(1.5)),
        ("requests_total", json!("3")),
        ("recent", json!({"seq": 1})),
    ] {
        assert_eq!(
            mutate(&|value| value["http"][field] = bad.clone()),
            Err("malformed JSON"),
            "{field}"
        );
    }
    assert_eq!(
        mutate(&|value| value["http"]["recent"][0]["status"] = json!(70_000)),
        Err("malformed JSON")
    );
    let deep = "[".repeat(10_000) + &"]".repeat(10_000);
    assert_eq!(parse_payload(deep.as_bytes()), Err("malformed JSON"));
    assert_eq!(
        parse_payload(&vec![b' '; MAX_BODY_BYTES + 1]),
        Err("response too large")
    );
}

#[test]
fn hostile_values_are_neutralized_and_lists_are_bounded() {
    let mut value = document();
    value["http"]["requests_total"] = json!(1_000);
    value["http"]["recent"] = Value::Array(
        (1..=200_u64)
            .map(|seq| {
                json!({
                    "seq": seq + 800,
                    "method": "GET\u{1b}[2J",
                    "path": format!("/x\u{1b}]0;owned\u{7}\u{202e}{}", "a".repeat(500)),
                    "status": if seq == 200 { 999 } else { 200 },
                    "duration_us": u64::MAX
                })
            })
            .collect(),
    );
    value["http"]["recent"].as_array_mut().unwrap().push(
        json!({"seq": 5_000, "method": "GET", "path": "/future", "status": 200, "duration_us": 1}),
    );
    value["database"]["recent_slow"] = Value::Array(
        (1..=40_u64)
            .map(|seq| json!({"seq": seq, "operation": "\u{9b}31m", "duration_us": 5}))
            .collect(),
    );
    value["database"]["slow_queries_total"] = json!(40);
    value["database"]["queries_total"] = json!(40);

    let snapshot = parse(&value).unwrap();
    // The newest 64 are kept; an invalid status and a sequence beyond the
    // total are dropped instead of trusted.
    assert_eq!(snapshot.http.recent.len(), MAX_REQUESTS - 2);
    for sample in &snapshot.http.recent {
        assert!(
            !sample.path.chars().any(char::is_control),
            "{:?}",
            sample.path
        );
        assert!(!sample.path.contains('\u{202e}'));
        assert!(sample.path.chars().count() <= 160);
        assert!(sample.path.ends_with('…'));
        assert!(!sample.method.contains('\u{1b}'));
        assert_eq!(sample.duration_us, 3_600_000_000);
    }
    let DatabaseReport::Observed { recent_slow, .. } = snapshot.database else {
        panic!("database should be observed");
    };
    assert_eq!(recent_slow.len(), MAX_SLOW_QUERIES);
    assert_eq!(recent_slow[0].operation, "?31m");
}

#[test]
fn database_and_queue_states_map_to_explicit_reports() {
    let with = |database: Value, queue: Value| {
        let mut value = document();
        value["database"] = database;
        value["queue"] = queue;
        let snapshot = parse(&value).unwrap();
        (snapshot.database, snapshot.queue)
    };
    assert_eq!(
        with(
            json!({"state": "unavailable", "reason": "subscriber_not_installed"}),
            json!({"state": "not_configured"})
        ),
        (
            DatabaseReport::SubscriberNotInstalled,
            QueueReport::NotConfigured
        )
    );
    assert_eq!(
        with(
            json!({"state": "unavailable", "reason": "orm_spans_filtered"}),
            json!({"state": "unavailable", "reason": "timeout"})
        ),
        (DatabaseReport::SpansFiltered, QueueReport::Timeout)
    );
    assert_eq!(
        with(
            json!({"state": "observed", "queries_total": 1, "slow_queries_total": 2, "slow_threshold_ms": 100}),
            json!({"state": "unavailable", "reason": "driver_error"})
        ),
        (DatabaseReport::Unknown, QueueReport::DriverError)
    );
    assert_eq!(
        with(json!({"state": "observed"}), json!({"state": "observed"})),
        (DatabaseReport::Unknown, QueueReport::Unknown)
    );
    assert_eq!(
        with(json!({"state": "sharded"}), json!({"state": "paused"})),
        (DatabaseReport::Unknown, QueueReport::Unknown)
    );
    let mut value = document();
    value.as_object_mut().unwrap().remove("database");
    value.as_object_mut().unwrap().remove("queue");
    let snapshot = parse(&value).unwrap();
    assert_eq!(snapshot.database, DatabaseReport::Unknown);
    assert_eq!(snapshot.queue, QueueReport::Unknown);
}

#[test]
fn cleaning_keeps_printable_text_and_marks_cuts() {
    assert_eq!(clean("/users/42", 20), "/users/42");
    assert_eq!(clean("/ação", 20), "/ação");
    assert_eq!(clean("abcdef", 4), "abc…");
    assert_eq!(clean("abcd", 4), "abcd");
    assert_eq!(clean("a\u{1b}b\u{7f}c\u{2066}d", 20), "a?b?c?d");
}

async fn serve(router: axum::Router) -> u16 {
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    port
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap()
}

#[tokio::test]
async fn polling_classifies_real_http_responses() {
    use axum::{http::header, response::IntoResponse, routing::get};

    let body = serde_json::to_vec(&document()).unwrap();
    let valid = body.clone();
    let port = serve(axum::Router::new().route(
        TELEMETRY_PATH,
        get(move || {
            let body = valid.clone();
            async move {
                (
                    [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
                    body,
                )
            }
        }),
    ))
    .await;
    assert!(matches!(
        poll(&client(), port).await,
        PollOutcome::Snapshot(_)
    ));

    let missing = serve(axum::Router::new()).await;
    assert_eq!(poll(&client(), missing).await, PollOutcome::NotServed);

    let failing = serve(axum::Router::new().route(
        TELEMETRY_PATH,
        get(|| async { axum::http::StatusCode::SERVICE_UNAVAILABLE }),
    ))
    .await;
    assert_eq!(poll(&client(), failing).await, PollOutcome::Unreachable);

    let html = body.clone();
    let html = serve(axum::Router::new().route(
        TELEMETRY_PATH,
        get(move || {
            let body = html.clone();
            async move { ([(header::CONTENT_TYPE, "text/html")], body) }
        }),
    ))
    .await;
    assert_eq!(
        poll(&client(), html).await,
        PollOutcome::Rejected("not a JSON response")
    );

    let large = serve(axum::Router::new().route(
        TELEMETRY_PATH,
        get(|| async {
            (
                [(header::CONTENT_TYPE, "application/json")],
                vec![b' '; MAX_BODY_BYTES + 10],
            )
                .into_response()
        }),
    ))
    .await;
    assert_eq!(
        poll(&client(), large).await,
        PollOutcome::Rejected("response too large")
    );

    // A chunked body without a length is cut at the same bound.
    let streamed = serve_chunked(64, 8 * 1024).await;
    assert_eq!(
        poll(&client(), streamed).await,
        PollOutcome::Rejected("response too large")
    );

    let redirect = serve(axum::Router::new().route(
        TELEMETRY_PATH,
        get(|| async { axum::response::Redirect::temporary("http://192.0.2.1/") }),
    ))
    .await;
    let no_redirects = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    assert_eq!(
        poll(&no_redirects, redirect).await,
        PollOutcome::Unreachable
    );

    // Port zero is never a reachable service (see `probe_port`'s test).
    assert_eq!(poll(&client(), 0).await, PollOutcome::Unreachable);
}

/// A raw HTTP/1.1 server that streams `chunks` chunks of `size` spaces.
async fn serve_chunked(chunks: usize, size: usize) -> u16 {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        let Ok((mut stream, _)) = listener.accept().await else {
            return;
        };
        let mut request = [0_u8; 2048];
        let _ = stream.read(&mut request).await;
        let mut response = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n\r\n".to_vec();
        for _ in 0..chunks {
            response.extend_from_slice(format!("{size:x}\r\n").as_bytes());
            response.extend(std::iter::repeat_n(b' ', size));
            response.extend_from_slice(b"\r\n");
        }
        response.extend_from_slice(b"0\r\n\r\n");
        let _ = stream.write_all(&response).await;
    });
    port
}
