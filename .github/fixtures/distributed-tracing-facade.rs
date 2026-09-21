use rullst::telemetry::distributed::{DistributedTelemetry, TelemetryConfig, TraceParent};
use tracing_subscriber::prelude::*;

#[test]
fn packaged_telemetry_exports_an_explicit_offline_observation() {
    let executor = rullst::async_runtime::tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    executor.block_on(async {
        let runtime = DistributedTelemetry::try_new(
            TelemetryConfig::try_new("archive-fixture", "mock_archive", ["archive.operation"])
                .unwrap(),
        )
        .unwrap();
        let subscriber = tracing_subscriber::registry().with(runtime.layer());
        tracing::subscriber::with_default(subscriber, || {
            let span = tracing::info_span!("archive.operation", secret = "not-exported");
            let context = TraceParent::capture(&span).unwrap();
            let mut headers = rullst::http::HeaderMap::new();
            context.inject(&mut headers).unwrap();
            assert_eq!(headers.get("traceparent").unwrap(), context.as_str());
        });
        runtime.flush().await.unwrap();
        let status = runtime.shutdown().await.unwrap();
        assert_eq!(status.offline_spans, 1);
        assert_eq!(status.accepted_batches, 0);
        assert_eq!(status.failed_batches, 0);
    });
}
