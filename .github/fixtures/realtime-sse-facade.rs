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

        // Over the limit, the stream only asks the browser to come back in
        // 5-10 seconds, and ends.
        let refused = sse.subscribe(channel.subscribe());
        assert_eq!(refused.status(), StatusCode::OK);
        assert_eq!(refused.headers()[header::CONTENT_TYPE], "text/event-stream");
        assert!(!refused.headers().contains_key(header::RETRY_AFTER));
        let refusal = to_bytes(refused.into_body(), 1024).await.unwrap();
        let refusal = std::str::from_utf8(&refusal).unwrap();
        let retry: u64 = refusal
            .strip_prefix(": connection limit reached\nretry: ")
            .and_then(|rest| rest.strip_suffix("\n\n"))
            .unwrap_or_else(|| panic!("unexpected refusal {refusal:?}"))
            .parse()
            .unwrap();
        assert!((5_000..=10_000).contains(&retry), "retry {retry}");

        channel.broadcast("stats", "<p>42</p>").unwrap();
        drop(channel);
        let body = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
        assert_eq!(&body[..], b"retry: 1000\n\nevent: stats\ndata: <p>42</p>\n\n");
        assert_eq!(sse.open_connections(), 0);

        sse.shutdown();
        assert!(sse.is_shut_down());
    });
}
