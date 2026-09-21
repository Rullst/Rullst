#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "live_recovery/app.rs"]
mod app;
#[path = "live_recovery/browser.rs"]
mod browser;
#[path = "live_recovery/protocol.rs"]
mod protocol;

#[tokio::test]
#[ignore = "requires the owned Chromium journey, explicitly executed in Linux CI"]
async fn chromium_recovers_across_disconnects_and_process_restart() {
    browser::exercise().await;
}

#[tokio::test]
#[ignore = "owned browser-service subprocess with bounded configuration on stdin"]
async fn fixture_server() {
    use std::io::Read;
    let mut input = Vec::new();
    std::io::stdin().take(8192).read_to_end(&mut input).unwrap();
    let config: app::Configuration = serde_json::from_slice(&input).unwrap();
    let server = app::Server::start(&config.database, config.port).await;
    tokio::fs::write(&config.ready, server.address.to_string())
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(180)).await;
    server.stop().await;
}
