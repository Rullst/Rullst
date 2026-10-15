use super::*;
use crate::{MessagingKeyring, MessagingStorageKey};
use std::path::PathBuf;

const UNKNOWN_ID: &str = "msg_00000000000000000000000000000000";

fn fixture() -> (PathBuf, String) {
    let path = std::env::temp_dir().join(format!(
        "rullst-webhook-unit-{}.sqlite",
        uuid::Uuid::new_v4().simple()
    ));
    let url = format!("sqlite://{}", path.to_string_lossy().replace('\\', "/"));
    (path, url)
}

fn cleanup(path: &std::path::Path) {
    for path in [
        path.to_path_buf(),
        path.with_extension("sqlite-wal"),
        path.with_extension("sqlite-shm"),
    ] {
        let _ = std::fs::remove_file(path);
    }
}

fn config(namespace: &str, retained: usize) -> WebhookConfig {
    let destination =
        WebhookDestination::approved_https("https://receiver.example/events").unwrap();
    WebhookConfig::new(namespace, destination, retained).unwrap()
}

async fn open(
    url: &str,
    config: WebhookConfig,
    clock: &ManualClock,
) -> Result<WebhookOutbox<ManualClock>> {
    let storage = MessagingKeyring::new(MessagingStorageKey::try_new("storage", [71; 32]).unwrap());
    let key = WebhookSigningKey::new("fixture", "mock_unit").unwrap();
    WebhookOutbox::open(url, config, key, storage, clock.clone()).await
}

#[tokio::test]
async fn enqueue_accepts_exactly_64_kib_of_json_and_maps_each_refusal() {
    let (path, url) = fixture();
    let clock = ManualClock::at(START);
    let store = open(&url, config("bodies", 4), &clock).await.unwrap();
    let largest = format!("\"{}\"", "a".repeat(65_534)).into_bytes();
    assert_eq!(largest.len(), 65_536);
    store.enqueue("largest", "ready", largest).await.unwrap();
    let refused = Some(WebhookError::InvalidInput("bounded JSON body"));
    let longer = format!("\"{}\"", "a".repeat(65_535)).into_bytes();
    assert_eq!(
        store.enqueue("longer", "ready", longer).await.err(),
        refused
    );
    assert_eq!(
        store
            .enqueue("text", "ready", b"not json".to_vec())
            .await
            .err(),
        refused
    );
    assert_eq!(
        store
            .enqueue("bad key", "ready", b"{}".to_vec())
            .await
            .err(),
        Some(WebhookError::InvalidInput("idempotency key"))
    );
    assert_eq!(format!("{store:?}"), "WebhookOutbox([REDACTED])");
    store.close().await;
    cleanup(&path);
}

#[tokio::test]
async fn failures_report_their_identity_and_closed_windows_are_invalid_leases() {
    let (path, url) = fixture();
    let clock = ManualClock::at(START);
    let store = open(&url, config("failures", 4), &clock).await.unwrap();
    let event = store
        .enqueue("expiring", "invoice.ready", b"{}".to_vec())
        .await
        .unwrap();
    clock.advance(86_400_000);
    assert!(matches!(
        store.dispatch_next("worker").await.unwrap(),
        WebhookDispatch::DeadLettered { attempt: 1, .. }
    ));
    let failures = store.failed(10).await.unwrap();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].delivery_id(), event.id().as_str());
    assert_eq!(failures[0].event_kind(), "invoice.ready");
    assert_eq!(failures[0].attempts(), 1);
    assert_eq!(failures[0].failure_code(), "webhook.expired");
    assert_eq!(failures[0].failed_at_ms(), 1_800_086_400_000);

    let invalid = Some(WebhookError::InvalidLease);
    assert_eq!(store.retry_failed(event.id().as_str()).await.err(), invalid);
    assert_eq!(store.retry_failed(UNKNOWN_ID).await.err(), invalid);
    assert_eq!(store.cancel(UNKNOWN_ID).await.err(), invalid);
    store.close().await;
    cleanup(&path);
}

#[tokio::test]
async fn purge_cutoffs_start_at_zero_and_negative_ones_are_clock_errors() {
    let (path, url) = fixture();
    let clock = ManualClock::at(START);
    let store = open(&url, config("cutoffs", 4), &clock).await.unwrap();
    assert_eq!(store.purge_terminal(0, 10).await, Ok(0));
    assert_eq!(store.purge_terminal(-1, 10).await, Err(WebhookError::Clock));
    store.close().await;
    cleanup(&path);
}

#[tokio::test]
async fn reopening_with_other_limits_is_a_configuration_error() {
    let (path, url) = fixture();
    let clock = ManualClock::at(START);
    let store = open(&url, config("limits", 4), &clock).await.unwrap();
    assert_eq!(
        open(&url, config("limits", 5), &clock).await.err(),
        Some(WebhookError::Configuration)
    );
    store.close().await;
    cleanup(&path);
}

#[tokio::test]
async fn closing_one_handle_closes_the_shared_storage() {
    let (path, url) = fixture();
    let clock = ManualClock::at(START);
    let store = open(&url, config("closing", 4), &clock).await.unwrap();
    let clone = store.clone();
    store.close().await;
    assert_eq!(
        clone
            .enqueue("after-close", "ready", b"{}".to_vec())
            .await
            .err(),
        Some(WebhookError::Storage)
    );
    cleanup(&path);
}
