//! Server-Sent Events responses for realtime channels.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::realtime::{Channel, TenantRealtime};
use crate::security::TenantContext;
use axum::body::Body;
use futures_util::StreamExt;
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

    let mut body = chunks(response);
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
    let mut body = chunks(sse.channel(&manager, "news"));

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
    let mut body = chunks(sse.channel(&manager, "room"));
    assert_eq!(sse.open_connections(), 1);

    sse.clone().shutdown();
    assert!(sse.is_shut_down());
    assert_eq!(next_text(&mut body).await, None);
    drop(body);
    assert_eq!(sse.open_connections(), 0);

    let refused = sse.channel(&manager, "late");
    assert_eq!(refused.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(refused.headers()[header::RETRY_AFTER], "5");
    // A refused request opens no channel, so there is nobody to publish to.
    assert!(manager.publish("late", "x", "y").is_err());
}

#[tokio::test]
async fn the_connection_limit_is_shared_by_clones() {
    let manager = BroadcastManager::new();
    let sse = RealtimeSse::new(RealtimeSseConfig::new().with_max_connections(1));
    let first = sse.channel(&manager, "room");
    assert_eq!(first.status(), StatusCode::OK);

    let other = sse.clone();
    assert_eq!(
        other.channel(&manager, "room").status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(other.open_connections(), 1);

    drop(first);
    assert_eq!(other.open_connections(), 0);
    assert_eq!(other.channel(&manager, "room").status(), StatusCode::OK);
}

#[tokio::test]
async fn a_connection_that_falls_behind_its_channel_ends() {
    let manager = BroadcastManager::new();
    let sse = RealtimeSse::new(RealtimeSseConfig::new());
    let mut body = chunks(sse.channel(&manager, "busy"));
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
    let mut body = chunks(sse.subscribe(channel.subscribe()));
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
    let mut body = chunks(sse.channel(&manager, "quiet"));

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
    let mut body = chunks(RealtimeSse::new(RealtimeSseConfig::new()).channel(&manager, "kept"));
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
    let mut body = chunks(sse.subscribe(acme.subscribe("orders").unwrap()));
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
    assert_eq!(defaults.max_lifetime(), Duration::from_secs(1800));
    assert_eq!(defaults.max_connections(), 1024);
    assert_eq!(defaults.max_payload_bytes(), 64 * 1024);

    let low = RealtimeSseConfig::new()
        .with_keep_alive(Duration::ZERO)
        .with_max_lifetime(Duration::ZERO)
        .with_max_connections(0)
        .with_max_payload_bytes(0);
    assert_eq!(low.keep_alive(), Duration::from_secs(1));
    assert_eq!(low.max_lifetime(), Duration::from_secs(1));
    assert_eq!(low.max_connections(), 1);
    assert_eq!(low.max_payload_bytes(), 1);

    let high = RealtimeSseConfig::new()
        .with_keep_alive(Duration::MAX)
        .with_max_lifetime(Duration::MAX)
        .with_max_connections(usize::MAX)
        .with_max_payload_bytes(usize::MAX);
    assert_eq!(high.keep_alive(), Duration::from_secs(300));
    assert_eq!(high.max_lifetime(), Duration::from_secs(86_400));
    assert_eq!(high.max_connections(), 65_536);
    assert_eq!(high.max_payload_bytes(), 64 * 1024);
    assert_eq!(RealtimeSse::new(high).config(), high);
}
