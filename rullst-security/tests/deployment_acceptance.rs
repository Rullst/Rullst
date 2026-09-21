//! Child process fixture, started only by the owned deployment runner.
#![cfg(feature = "redis-rate-limit")]
#[path = "deployment_support/application.rs"]
mod application;

#[tokio::test]
#[ignore = "owned disposable proxy/Redis deployment runner only"]
async fn owned_application_process() {
    application::run().await;
}
