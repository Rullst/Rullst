#![cfg(feature = "telemetry")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "distributed_tracing/collector.rs"]
mod collector;
#[path = "distributed_tracing/protocol.rs"]
mod protocol;

#[tokio::test]
#[ignore = "owned legacy subscriber subprocess with explicit collector and receipt file"]
async fn legacy_process() {
    use std::io::Read;
    let mut input = String::new();
    std::io::stdin()
        .take(8192)
        .read_to_string(&mut input)
        .unwrap();
    let receipt: std::path::PathBuf = serde_json::from_str(&input).unwrap();
    rullst_core::telemetry::init_telemetry().unwrap();
    {
        let _span = tracing::info_span!("legacy.fixture");
    }
    tokio::time::timeout(std::time::Duration::from_secs(20), async {
        while !receipt.exists() {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
}
