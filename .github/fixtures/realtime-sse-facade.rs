use rullst::realtime::{Channel, RealtimeSse, RealtimeSseConfig};
use rullst::web::axum::body::to_bytes;
use rullst::web::axum::http::{StatusCode, header};

// This consumer proves that the packaged facade, without optional features,
// streams a realtime channel as Server-Sent Events within its limits. The
// framework's own tests own keep-alive, lifetime, lag and shutdown behaviour.
#[test]
fn packaged_facade_streams_realtime_channels_as_server_sent_events() {
    let runtime = rullst::async_runtime::tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let sse = RealtimeSse::new(RealtimeSseConfig::new().with_max_connections(1));
        let channel = Channel::new("dashboard", 4);
        let response = sse.subscribe(channel.subscribe());
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "text/event-stream");
        let refused = sse.subscribe(channel.subscribe());
        assert_eq!(refused.status(), StatusCode::SERVICE_UNAVAILABLE);

        channel.broadcast("stats", "<p>42</p>").unwrap();
        drop(channel);
        let body = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
        assert_eq!(&body[..], b"event: stats\ndata: <p>42</p>\n\n");
        assert_eq!(sse.open_connections(), 0);

        sse.shutdown();
        assert!(sse.is_shut_down());
    });
}
