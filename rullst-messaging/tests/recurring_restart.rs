#![cfg(feature = "schedules-postgres")]
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[allow(dead_code, unused_imports)]
#[path = "recurring/support.rs"]
mod support;
use std::process::Stdio;
use support::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(serde::Serialize, serde::Deserialize)]
struct Receipt {
    namespace: String,
    time: i64,
}
#[derive(serde::Deserialize)]
struct Input {
    url: String,
    receipt: Receipt,
}
#[tokio::test]
#[ignore = "private configuration arrives through parent-owned stdin"]
async fn recurring_process_worker() {
    let mut input = Vec::new();
    tokio::io::stdin()
        .take(16384)
        .read_to_end(&mut input)
        .await
        .unwrap();
    let input: Input = serde_json::from_slice(&input).unwrap();
    let clock = ManualClock(Arc::new(AtomicI64::new(input.receipt.time)));
    let store = PostgresRecurringStore::connect(
        &input.url,
        config(&input.receipt.namespace),
        keys(),
        clock.clone(),
    )
    .await
    .unwrap();
    let states = store.occurrences(None, 10).await.unwrap();
    assert_eq!(states.len(), 1);
    if states[0].state() == OccurrenceState::Leased {
        let lease = store.claim(1).await.unwrap().remove(0);
        assert_eq!(lease.metadata().attempts(), 2);
        let broker = InMemoryBroker::with_clock(BrokerConfig::try_new("child").unwrap(), clock);
        store.relay(&lease, &broker).await.unwrap();
    } else {
        assert_eq!(states[0].state(), OccurrenceState::Published);
    }
    assert!(store.claim(1).await.unwrap().is_empty());
    assert_eq!(
        store.occurrences(None, 1).await.unwrap()[0].state(),
        OccurrenceState::Published
    );
    assert!(store.tick(1).await.unwrap().is_empty());
    store.close().await;
}
async fn child(url: &str, receipt: &Receipt) {
    let mut child = tokio::process::Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "recurring_process_worker"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&serde_json::json!({"url":url,"receipt":receipt})).unwrap())
        .await
        .unwrap();
    let result = tokio::time::timeout(Duration::from_secs(30), child.wait_with_output())
        .await
        .unwrap()
        .unwrap();
    assert!(
        result.status.success(),
        "owned recurring worker failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}
#[tokio::test]
#[ignore = "requires wrapper-owned PostgreSQL and server restart"]
async fn postgres_recurring_process_restart() {
    let url = url();
    let path = std::env::var("RULLST_RECURRING_RESTART_RECEIPT").unwrap();
    match std::env::var("RULLST_RECURRING_RESTART_PHASE")
        .unwrap()
        .as_str()
    {
        "exercise" => {
            let clock = ManualClock::new();
            let namespace = unique();
            let store = open(&url, &namespace, &clock).await;
            store
                .create(definition("restart", &clock, MissedRunPolicy::CatchUp))
                .await
                .unwrap();
            store.tick(1).await.unwrap();
            assert_eq!(store.claim(1).await.unwrap().len(), 1);
            store.close().await;
            clock.advance(2001);
            let receipt = Receipt {
                namespace,
                time: clock.now_millis().unwrap(),
            };
            child(&url, &receipt).await;
            std::fs::write(path, serde_json::to_vec(&receipt).unwrap()).unwrap();
        }
        "restart" => {
            let receipt = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
            child(&url, &receipt).await;
        }
        _ => panic!("wrapper phase missing"),
    }
}
