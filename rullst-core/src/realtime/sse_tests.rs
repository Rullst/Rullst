//! Server-Sent Events responses for realtime channels.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::realtime::{Channel, TenantRealtime};
use crate::security::TenantContext;
use axum::body::Body;
use axum::http::{StatusCode, header};
use std::sync::Arc;
use std::time::Duration;

type Chunks = futures_util::stream::BoxStream<'static, Result<axum::body::Bytes, axum::Error>>;

fn chunks(response: Response) -> Chunks {
    Body::new(response.into_body()).into_data_stream().boxed()
}

/// The next chunk of the body as text, or `None` once the stream ended.
async fn next_text(body: &mut Chunks) -> Option<String> {
    let chunk = tokio::time::timeout(Duration::from_secs(5), body.next())
        .await
        .expect("the stream answers in time")?;
    Some(String::from_utf8(chunk.expect("chunk").to_vec()).expect("UTF-8 chunk"))
}

/// The body of an admitted stream, after its leading `retry:` field.
async fn admitted(response: Response) -> Chunks {
    assert_eq!(response.status(), StatusCode::OK);
    let mut body = chunks(response);
    assert_eq!(
        next_text(&mut body).await.as_deref(),
        Some("retry: 1000\n\n")
    );
    body
}

/// The comment and the `retry:` milliseconds of a refused request, whose
/// stream then ends.
async fn refusal(response: Response) -> (String, u64) {
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CONTENT_TYPE],
        "text/event-stream"
    );
    assert_eq!(response.headers()["x-accel-buffering"], "no");
    assert!(!response.headers().contains_key(header::RETRY_AFTER));
    let mut body = chunks(response);
    let text = next_text(&mut body).await.expect("a refusal event");
    assert_eq!(next_text(&mut body).await, None, "the refusal ends at once");
    let (comment, retry) = text
        .strip_prefix(": ")
        .and_then(|rest| rest.strip_suffix("\n\n"))
        .and_then(|rest| rest.split_once("\nretry: "))
        .unwrap_or_else(|| panic!("unexpected refusal {text:?}"));
    (
        comment.to_owned(),
        retry.parse().expect("retry milliseconds"),
    )
}

#[tokio::test]
async fn published_messages_become_named_events() {
    let manager = BroadcastManager::new();
    let sse = RealtimeSse::new(RealtimeSseConfig::new());
    let response = sse.channel(&manager, "dashboard");

    assert_eq!(response.status(), StatusCode::OK);
    let headers = response.headers();
    assert_eq!(headers[header::CONTENT_TYPE], "text/event-stream");
    assert_eq!(headers[header::CACHE_CONTROL], "no-cache");
    assert_eq!(headers["x-accel-buffering"], "no");

    let mut body = admitted(response).await;
    assert_eq!(
        manager.publish("dashboard", "stats", "<p id=\"stats\">1</p>\n<p>2</p>"),
        Ok(1)
    );
    assert_eq!(
        next_text(&mut body).await.as_deref(),
        Some("event: stats\ndata: <p id=\"stats\">1</p>\ndata: <p>2</p>\n\n")
    );
}

#[tokio::test]
async fn unsendable_messages_are_skipped() {
    let manager = BroadcastManager::new();
    let sse = RealtimeSse::new(RealtimeSseConfig::new().with_max_payload_bytes(8));
    let mut body = admitted(sse.channel(&manager, "news")).await;

    for (event, payload) in [
        ("line\nbreak", "ok"),
        ("cr\rname", "ok"),
        ("", "ok"),
        ("spaced name", "ok"),
        ("big", "123456789"),
    ] {
        manager.publish("news", event, payload).unwrap();
    }
    manager.publish("news", "news/item:1", "12345678").unwrap();

    assert_eq!(
        next_text(&mut body).await.as_deref(),
        Some("event: news/item:1\ndata: 12345678\n\n")
    );
}

#[tokio::test]
async fn shutdown_ends_open_streams_and_refuses_new_ones() {
    let manager = BroadcastManager::new();
    let sse = RealtimeSse::new(RealtimeSseConfig::new());
    let mut body = admitted(sse.channel(&manager, "room")).await;
    assert_eq!(sse.open_connections(), 1);

    sse.clone().shutdown();
    assert!(sse.is_shut_down());
    assert_eq!(next_text(&mut body).await, None);
    drop(body);
    assert_eq!(sse.open_connections(), 0);

    let (comment, retry) = refusal(sse.channel(&manager, "late")).await;
    assert_eq!(comment, "shutting down");
    assert!((5_000..=10_000).contains(&retry), "{retry}");
    // A refused request opens no channel, so there is nobody to publish to.
    assert!(manager.publish("late", "x", "y").is_err());
    let channel = Channel::new("solo", 4);
    let (comment, _) = refusal(sse.subscribe(channel.subscribe())).await;
    assert_eq!(comment, "shutting down");
    assert_eq!(sse.open_connections(), 0);
}

#[tokio::test]
async fn the_connection_limit_is_shared_by_clones() {
    let manager = BroadcastManager::new();
    let sse = RealtimeSse::new(RealtimeSseConfig::new().with_max_connections(1));
    let first = sse.channel(&manager, "room");
    assert_eq!(first.status(), StatusCode::OK);

    let other = sse.clone();
    let (comment, retry) = refusal(other.channel(&manager, "room")).await;
    assert_eq!(comment, "connection limit reached");
    assert!((5_000..=10_000).contains(&retry), "{retry}");
    let channel = Channel::new("solo", 4);
    let (comment, _) = refusal(other.subscribe(channel.subscribe())).await;
    assert_eq!(comment, "connection limit reached");
    assert_eq!(other.open_connections(), 1);

    drop(first);
    assert_eq!(other.open_connections(), 0);
    let mut body = admitted(other.channel(&manager, "room")).await;
    manager.publish("room", "back", "in").unwrap();
    assert_eq!(
        next_text(&mut body).await.as_deref(),
        Some("event: back\ndata: in\n\n")
    );
}

#[tokio::test]
async fn streams_start_with_the_reconnect_delay_and_refusals_stay_longer() {
    let manager = BroadcastManager::new();
    let config = RealtimeSseConfig::new()
        .with_reconnect_delay(Duration::from_millis(2_500))
        .with_max_connections(1);
    let sse = RealtimeSse::new(config);
    let mut body = chunks(sse.channel(&manager, "room"));
    assert_eq!(
        next_text(&mut body).await.as_deref(),
        Some("retry: 2500\n\n")
    );

    // Refused requests wait at least 5 seconds, plus up to as much again.
    let mut retries = Vec::new();
    for _ in 0..20 {
        let (_, retry) = refusal(sse.channel(&manager, "room")).await;
        assert!((5_000..=10_000).contains(&retry), "{retry}");
        retries.push(retry);
    }
    // Spread over the whole extra range, not a fixed or near-fixed value.
    let spread = retries.iter().max().unwrap() - retries.iter().min().unwrap();
    assert!(spread > 1_000, "{retries:?}");

    // A reconnect delay above 5 seconds is the floor instead.
    let slow = RealtimeSse::new(
        RealtimeSseConfig::new()
            .with_reconnect_delay(Duration::from_secs(60))
            .with_max_connections(1),
    );
    let _open = slow.channel(&manager, "room");
    let (_, retry) = refusal(slow.channel(&manager, "room")).await;
    assert!((60_000..=120_000).contains(&retry), "{retry}");
}

#[tokio::test]
async fn a_connection_that_falls_behind_its_channel_ends() {
    let manager = BroadcastManager::new();
    let sse = RealtimeSse::new(RealtimeSseConfig::new());
    let mut body = admitted(sse.channel(&manager, "busy")).await;
    // Manager channels hold 100 messages; the first read finds the gap.
    for index in 0..150 {
        manager.publish("busy", "tick", &index.to_string()).unwrap();
    }
    assert_eq!(next_text(&mut body).await, None);
}

#[tokio::test]
async fn a_closed_channel_ends_the_stream() {
    let channel = Channel::new("solo", 4);
    let sse = RealtimeSse::new(RealtimeSseConfig::new());
    let mut body = admitted(sse.subscribe(channel.subscribe())).await;
    channel.broadcast("hello", "world").unwrap();
    drop(channel);

    assert_eq!(
        next_text(&mut body).await.as_deref(),
        Some("event: hello\ndata: world\n\n")
    );
    assert_eq!(next_text(&mut body).await, None);
}

#[tokio::test]
async fn idle_streams_send_keep_alive_comments_and_end_after_their_lifetime() {
    let manager = BroadcastManager::new();
    let config = RealtimeSseConfig::new()
        .with_keep_alive(Duration::from_secs(1))
        .with_max_lifetime(Duration::from_secs(2));
    let sse = RealtimeSse::new(config);
    let mut body = admitted(sse.channel(&manager, "quiet")).await;

    assert_eq!(
        next_text(&mut body).await.as_deref(),
        Some(": keep-alive\n\n")
    );
    let mut rest = Vec::new();
    while let Some(text) = next_text(&mut body).await {
        rest.push(text);
    }
    assert!(
        rest.iter().all(|text| text == ": keep-alive\n\n"),
        "{rest:?}"
    );
}

#[tokio::test]
async fn dropping_every_handler_keeps_open_streams_running() {
    let manager = BroadcastManager::new();
    let mut body =
        admitted(RealtimeSse::new(RealtimeSseConfig::new()).channel(&manager, "kept")).await;
    manager.publish("kept", "still", "open").unwrap();
    assert_eq!(
        next_text(&mut body).await.as_deref(),
        Some("event: still\ndata: open\n\n")
    );
}

#[tokio::test]
async fn tenant_subscriptions_stream_only_their_tenant() {
    let manager = Arc::new(BroadcastManager::new());
    let acme = TenantRealtime::from_context(
        Arc::clone(&manager),
        &TenantContext::try_new("acme").unwrap(),
    );
    let globex = TenantRealtime::from_context(
        Arc::clone(&manager),
        &TenantContext::try_new("globex").unwrap(),
    );
    let sse = RealtimeSse::new(RealtimeSseConfig::new());
    let mut body = admitted(sse.subscribe(acme.subscribe("orders").unwrap())).await;
    let _other = globex.subscribe("orders").unwrap();

    globex.publish("orders", "order", "globex").unwrap();
    acme.publish("orders", "order", "acme").unwrap();
    assert_eq!(
        next_text(&mut body).await.as_deref(),
        Some("event: order\ndata: acme\n\n")
    );
}

#[test]
fn every_limit_is_clamped_into_its_range() {
    let defaults = RealtimeSseConfig::default();
    assert_eq!(defaults, RealtimeSseConfig::new());
    assert_eq!(defaults.keep_alive(), Duration::from_secs(15));
    assert_eq!(defaults.reconnect_delay(), Duration::from_secs(1));
    assert_eq!(defaults.max_lifetime(), Duration::from_secs(1800));
    assert_eq!(defaults.max_connections(), 1024);
    assert_eq!(defaults.max_payload_bytes(), 64 * 1024);

    let low = RealtimeSseConfig::new()
        .with_keep_alive(Duration::ZERO)
        .with_reconnect_delay(Duration::ZERO)
        .with_max_lifetime(Duration::ZERO)
        .with_max_connections(0)
        .with_max_payload_bytes(0);
    assert_eq!(low.keep_alive(), Duration::from_secs(1));
    assert_eq!(low.reconnect_delay(), Duration::from_millis(100));
    assert_eq!(low.max_lifetime(), Duration::from_secs(1));
    assert_eq!(low.max_connections(), 1);
    assert_eq!(low.max_payload_bytes(), 1);

    let high = RealtimeSseConfig::new()
        .with_keep_alive(Duration::MAX)
        .with_reconnect_delay(Duration::MAX)
        .with_max_lifetime(Duration::MAX)
        .with_max_connections(usize::MAX)
        .with_max_payload_bytes(usize::MAX);
    assert_eq!(high.keep_alive(), Duration::from_secs(300));
    assert_eq!(high.reconnect_delay(), Duration::from_secs(300));
    assert_eq!(high.max_lifetime(), Duration::from_secs(86_400));
    assert_eq!(high.max_connections(), 65_536);
    assert_eq!(high.max_payload_bytes(), 64 * 1024);
    assert_eq!(RealtimeSse::new(high).config(), high);
}
