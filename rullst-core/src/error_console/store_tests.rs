//! Bounds, expiry and content of the `cargo rullst ai fix` error store.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::store::{ErrorContext, MAX_ENTRIES, Store, TTL, project_frames, valid_id};
use std::time::{Duration, Instant};

const BACKTRACE: &str = "   0: std::panicking::begin_panic\n             at /rustc/abc/library/std/src/panicking.rs:689:12\n   1: tokio::runtime::task::harness::poll\n             at /home/dev/.cargo/registry/src/index.crates.io-1/tokio-1.52.3/src/runtime/task/harness.rs:473:19\n   2: app::controllers::users::show\n             at ./src/controllers/users.rs:42:9\n   3: app::main\n             at ./src/main.rs:7:5\n   4: __rust_begin_short_backtrace\n";

fn record(store: &mut Store, now: Instant, message: &str) -> String {
    store.record(
        now,
        message,
        Some(("src/controllers/users.rs".to_string(), 42)),
        Some(BACKTRACE),
        "GET",
        "/users/7",
    )
}

#[test]
fn entries_hold_only_the_console_view_of_a_panic() {
    let mut store = Store::default();
    let now = Instant::now();
    let id = record(&mut store, now, "boom");
    assert!(valid_id(&id), "{id}");
    let context = store.get(now, &id).unwrap();
    assert_eq!(
        context,
        ErrorContext {
            schema: "rullst.error-context.v1",
            id: id.clone(),
            message: "boom".to_string(),
            file: Some("src/controllers/users.rs".to_string()),
            line: Some(42),
            backtrace: vec![
                "app::controllers::users::show at ./src/controllers/users.rs:42:9".to_string(),
                "app::main at ./src/main.rs:7:5".to_string(),
            ],
            method: "GET".to_string(),
            path: "/users/7".to_string(),
            expires_in_seconds: TTL.as_secs(),
        }
    );
    // The served document has exactly these keys: no headers, cookies,
    // query string or body exist to leak.
    let json = serde_json::to_value(&context).unwrap();
    let mut keys: Vec<&str> = json
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "backtrace",
            "expires_in_seconds",
            "file",
            "id",
            "line",
            "message",
            "method",
            "path",
            "schema"
        ]
    );
}

#[test]
fn the_store_is_bounded_in_number_and_size() {
    let mut store = Store::default();
    let now = Instant::now();
    let first = record(&mut store, now, "first");
    let ids: Vec<String> = (0..MAX_ENTRIES)
        .map(|index| record(&mut store, now, &format!("panic {index}")))
        .collect();
    assert_eq!(store.len(), MAX_ENTRIES);
    assert!(
        store.get(now, &first).is_none(),
        "the oldest entry is dropped"
    );
    assert!(store.get(now, &ids[0]).is_some());

    let long = "x".repeat(10_000);
    let deep: String = (0..64)
        .map(|index| format!("  {index}: app::frame{index}\n      at ./src/f{index}.rs:1:1\n"))
        .collect();
    let id = store.record(
        now,
        &long,
        Some((long.clone(), 1)),
        Some(&deep),
        &long,
        &long,
    );
    let context = store.get(now, &id).unwrap();
    assert!(context.message.len() <= 2 * 1024 + 3);
    assert!(context.file.unwrap().len() <= 512 + 3);
    assert!(context.path.len() <= 512 + 3);
    assert!(context.method.len() <= 16 + 3);
    assert_eq!(context.backtrace.len(), 16);
    assert_eq!(project_frames(""), Vec::<String>::new());
}

#[test]
fn entries_expire() {
    let mut store = Store::default();
    let now = Instant::now();
    let id = record(&mut store, now, "boom");
    let later = now + Duration::from_secs(600);
    assert_eq!(
        store.get(later, &id).unwrap().expires_in_seconds,
        TTL.as_secs() - 600
    );
    assert!(store.get(now + TTL, &id).is_none());
    assert_eq!(store.len(), 0, "expired entries are removed");
}

#[test]
fn ids_are_32_lowercase_hex_characters() {
    assert!(valid_id("0123456789abcdef0123456789abcdef"));
    for id in [
        "",
        "0123456789ABCDEF0123456789ABCDEF",
        "0123456789abcdef0123456789abcde",
        "../../etc/passwd",
        "0123456789abcdef0123456789abcdeg",
    ] {
        assert!(!valid_id(id), "{id}");
    }
}
