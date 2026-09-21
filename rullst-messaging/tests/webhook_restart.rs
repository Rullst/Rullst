#![cfg(feature = "webhooks")]
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[allow(dead_code)]
#[path = "webhook/server.rs"]
mod server;
#[allow(dead_code)]
#[path = "webhook/support.rs"]
mod support;
use std::process::Stdio;
use support::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(serde::Serialize, serde::Deserialize)]
struct Input {
    database: String,
    namespace: String,
    destination: String,
    time: i64,
    deliver: bool,
}
#[tokio::test]
#[ignore = "private configuration arrives through owning test stdin"]
async fn webhook_process_worker() {
    let mut bytes = Vec::new();
    tokio::io::stdin()
        .take(16384)
        .read_to_end(&mut bytes)
        .await
        .unwrap();
    let input: Input = serde_json::from_slice(&bytes).unwrap();
    let clock = ManualClock(Arc::new(AtomicI64::new(input.time)));
    let store = open(
        &input.database,
        &input.namespace,
        WebhookDestination::loopback_test(input.destination).unwrap(),
        &clock,
    )
    .await;
    let result = store.dispatch_next("child").await.unwrap();
    if input.deliver {
        assert!(matches!(
            result,
            WebhookDispatch::Accepted {
                offline: false,
                status: 200,
                ..
            }
        ));
    } else {
        assert_eq!(result, WebhookDispatch::Idle);
    }
    store.close().await;
}
async fn child(input: &Input) {
    let mut process = tokio::process::Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "webhook_process_worker"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    process
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(input).unwrap())
        .await
        .unwrap();
    let result = tokio::time::timeout(Duration::from_secs(20), process.wait_with_output())
        .await
        .unwrap()
        .unwrap();
    assert!(
        result.status.success(),
        "owned webhook child failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}
#[tokio::test]
async fn fresh_process_delivers_and_another_process_observes_durable_acknowledgement() {
    let clock = ManualClock::new();
    let receiver = server::Receiver::start(clock.clone(), false).await;
    let (path, url) = fixture();
    let namespace = unique();
    let store = open(&url, &namespace, receiver.destination(), &clock).await;
    let event = store
        .enqueue("restart", "ready", b"{\"restart\":true}".to_vec())
        .await
        .unwrap();
    store.close().await;
    let mut input = Input {
        database: url.clone(),
        namespace: namespace.clone(),
        destination: receiver.url.clone(),
        time: clock.now_millis().unwrap(),
        deliver: true,
    };
    child(&input).await;
    input.deliver = false;
    child(&input).await;
    assert_eq!(receiver.effects.lock().await.len(), 1);
    assert!(receiver.effects.lock().await.contains(event.id().as_str()));
    let reopened = open(&url, &namespace, receiver.destination(), &clock).await;
    let duplicate = reopened
        .enqueue("restart", "ready", b"{\"restart\":true}".to_vec())
        .await
        .unwrap();
    assert_eq!(duplicate.id(), event.id());
    assert!(duplicate.is_duplicate());
    reopened.close().await;
    cleanup(&path);
}
