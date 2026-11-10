//! The facade exposes Server-Sent Events for realtime channels at
//! `rullst::realtime` and the crate root, and a route that returns one
//! streams the channel's messages.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use rullst::realtime::{BroadcastManager, Channel, RealtimeSseConfig};
use rullst::web::axum::{self, body::Body, extract::State, http, response::Response};
use tower::ServiceExt;

#[tokio::test]
async fn the_root_and_module_paths_name_the_same_types() {
    let sse: rullst::realtime::RealtimeSse =
        rullst::RealtimeSse::new(rullst::RealtimeSseConfig::new());
    assert_eq!(sse.config(), RealtimeSseConfig::default());

    let channel = Channel::new("dashboard", 4);
    let response = sse.subscribe(channel.subscribe());
    assert_eq!(response.status(), http::StatusCode::OK);
    channel.broadcast("stats", "<p>42</p>").unwrap();
    drop(channel);
    let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap();
    assert_eq!(&body[..], b"event: stats\ndata: <p>42</p>\n\n");
    assert_eq!(sse.open_connections(), 0);
}

#[derive(Clone)]
struct AppState {
    channels: Arc<BroadcastManager>,
    sse: rullst::RealtimeSse,
}

async fn events(State(state): State<AppState>) -> Response {
    state.sse.channel(&state.channels, "dashboard")
}

#[tokio::test]
async fn a_route_streams_published_fragments_until_shutdown() {
    let state = AppState {
        channels: Arc::new(BroadcastManager::new()),
        sse: rullst::RealtimeSse::new(RealtimeSseConfig::new().with_max_connections(1)),
    };
    let app = rullst::Router::new()
        .merge_axum(
            axum::Router::new()
                .route("/events", axum::routing::get(events))
                .with_state(state.clone()),
        )
        .into_axum();
    let request = || {
        http::Request::get("/events")
            .header(http::header::ACCEPT, "text/event-stream")
            .body(Body::empty())
            .unwrap()
    };

    let response = app.clone().oneshot(request()).await.unwrap();
    assert_eq!(response.status(), http::StatusCode::OK);
    assert_eq!(
        response.headers()[http::header::CONTENT_TYPE],
        "text/event-stream"
    );
    // The limit of one connection refuses the second stream.
    let refused = app.clone().oneshot(request()).await.unwrap();
    assert_eq!(refused.status(), http::StatusCode::SERVICE_UNAVAILABLE);
    assert!(refused.headers().contains_key(http::header::RETRY_AFTER));

    assert_eq!(
        state.channels.publish("dashboard", "stats", "<p>7</p>"),
        Ok(1)
    );
    let mut body = response.into_body().into_data_stream();
    let first = futures_util::StreamExt::next(&mut body)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(&first[..], b"event: stats\ndata: <p>7</p>\n\n");

    state.sse.shutdown();
    assert!(futures_util::StreamExt::next(&mut body).await.is_none());
    let after = app.oneshot(request()).await.unwrap();
    assert_eq!(after.status(), http::StatusCode::SERVICE_UNAVAILABLE);
}
