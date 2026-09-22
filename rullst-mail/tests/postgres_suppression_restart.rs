#![cfg(feature = "postgres")]
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[allow(dead_code, unused_imports)]
#[path = "suppression_postgres/support.rs"]
mod support;
use std::process::Stdio;
use support::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(serde::Serialize, serde::Deserialize)]
struct Receipt {
    namespace: String,
    time: u64,
}
#[derive(serde::Deserialize)]
struct Input {
    url: String,
    receipt: Receipt,
}

#[tokio::test]
#[ignore = "private configuration must arrive on stdin from the owning test"]
async fn mail_suppression_process_worker() {
    let mut input = Vec::new();
    tokio::io::stdin()
        .take(16384)
        .read_to_end(&mut input)
        .await
        .unwrap();
    let input: Input = serde_json::from_slice(&input).unwrap();
    let store =
        PostgresSuppressionStore::connect(&input.url, key(), config(&input.receipt.namespace))
            .await
            .unwrap();
    let record = store.lookup("restart@example.com").await.unwrap().unwrap();
    assert_eq!(record.reason(), SuppressionReason::HardBounce);
    store
        .record(event(
            "initial",
            "restart@example.com",
            SuppressionReason::HardBounce,
            input.receipt.time,
        ))
        .await
        .unwrap();
    store
        .record(event(
            "child",
            "child@example.com",
            SuppressionReason::SpamComplaint,
            input.receipt.time,
        ))
        .await
        .unwrap();
    assert_eq!(store.snapshot().await.unwrap().recipients(), 2);
    assert_eq!(store.snapshot().await.unwrap().events(), 2);
    let (driver, inbox) = MemoryDriver::isolated();
    let guard = SuppressionGuard::new(driver, store.clone());
    assert!(matches!(
        guard.send(&message("restart@example.com")).await,
        Err(MailError::SuppressedRecipient { .. })
    ));
    assert!(inbox.lock().unwrap().is_empty());
    store.close().await;
}

async fn child(url: &str, receipt: &Receipt) {
    let mut child = tokio::process::Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "mail_suppression_process_worker"])
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
    let output = tokio::time::timeout(std::time::Duration::from_secs(20), child.wait_with_output())
        .await
        .unwrap()
        .unwrap();
    assert!(
        output.status.success(),
        "owned suppression worker failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let reopened = PostgresSuppressionStore::connect(url, key(), config(&receipt.namespace))
        .await
        .unwrap();
    assert_eq!(
        reopened
            .lookup("child@example.com")
            .await
            .unwrap()
            .unwrap()
            .reason(),
        SuppressionReason::SpamComplaint
    );
    reopened.close().await;
}

#[tokio::test]
#[ignore = "requires wrapper-owned PostgreSQL and an explicit server restart"]
async fn postgres_suppression_process_restart() {
    let url = url();
    let path = std::env::var("RULLST_MAIL_RESTART_RECEIPT").unwrap();
    match std::env::var("RULLST_MAIL_RESTART_PHASE").unwrap().as_str() {
        "exercise" => {
            let receipt = Receipt {
                namespace: unique(),
                time: now() - 60,
            };
            let store =
                PostgresSuppressionStore::initialize(&url, key(), config(&receipt.namespace))
                    .await
                    .unwrap();
            store
                .record(event(
                    "initial",
                    "restart@example.com",
                    SuppressionReason::HardBounce,
                    receipt.time,
                ))
                .await
                .unwrap();
            store.close().await;
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
