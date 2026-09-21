#![cfg(any(feature = "api-tokens-sqlite", feature = "api-tokens-postgres"))]
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[allow(dead_code, unused_imports)]
#[path = "api_tokens/support.rs"]
mod support;
use std::process::Stdio;
use support::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(serde::Serialize, serde::Deserialize)]
struct Receipt {
    namespace: String,
    rotated_from: String,
    active: String,
    revoked: String,
    subject: String,
    email: String,
    password: String,
}
#[derive(serde::Deserialize)]
struct Worker {
    url: String,
    receipt: Receipt,
    revoke: bool,
    expect_active: bool,
}

#[tokio::test]
#[ignore = "owned subprocess; private fixture configuration must arrive on stdin"]
async fn api_token_restart_worker() {
    let mut input = Vec::new();
    tokio::io::stdin()
        .take(16384)
        .read_to_end(&mut input)
        .await
        .unwrap();
    let worker: Worker = serde_json::from_slice(&input).unwrap();
    let receipt = worker.receipt;
    let service = ApiTokenService::connect(&worker.url, keys(), config(&receipt.namespace))
        .await
        .unwrap();
    let clock = Clock::new();
    for invalid in [&receipt.rotated_from, &receipt.revoked] {
        assert_eq!(
            service.verify(invalid, &read(), &clock).await.unwrap_err(),
            RecoveryError::InvalidAction
        );
    }
    let principal = service.verify(&receipt.active, &read(), &clock).await;
    if worker.expect_active {
        let principal = principal.unwrap();
        assert_eq!(principal.subject(), receipt.subject);
        assert_eq!(principal.metadata().revision(), 2);
        if worker.revoke {
            let owner = service
                .accounts()
                .authenticate(&receipt.email, &receipt.password)
                .await
                .unwrap()
                .unwrap();
            assert!(
                service
                    .revoke(&owner, principal.metadata().id(), &clock)
                    .await
                    .unwrap()
            );
        }
    } else {
        assert_eq!(principal.unwrap_err(), RecoveryError::InvalidAction);
    }
    service.close().await;
}

async fn child(url: &str, receipt: &Receipt, revoke: bool, expect_active: bool) {
    let value = serde_json::json!({"url":url,"receipt":receipt,"revoke":revoke,"expect_active":expect_active});
    let mut child = tokio::process::Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "api_token_restart_worker"])
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
        .write_all(&serde_json::to_vec(&value).unwrap())
        .await
        .unwrap();
    let result = tokio::time::timeout(std::time::Duration::from_secs(20), child.wait_with_output())
        .await
        .unwrap()
        .unwrap();
    assert!(
        result.status.success(),
        "owned API-token worker failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

async fn exercise(url: &str) -> Receipt {
    let namespace = unique();
    let clock = Clock::new();
    let service = ApiTokenService::initialize(url, keys(), config(&namespace))
        .await
        .unwrap();
    let (owner, email, password) = account(&service, &clock).await;
    let first = service
        .issue(&owner, read(), label(), 600, &clock)
        .await
        .unwrap();
    let active = service
        .rotate(&owner, first.metadata().id(), 1, 600, &clock)
        .await
        .unwrap();
    let revoked = service
        .issue(&owner, read(), label(), 600, &clock)
        .await
        .unwrap();
    service
        .revoke(&owner, revoked.metadata().id(), &clock)
        .await
        .unwrap();
    let receipt = Receipt {
        namespace,
        rotated_from: first.expose_bearer().to_owned(),
        active: active.expose_bearer().to_owned(),
        revoked: revoked.expose_bearer().to_owned(),
        subject: owner.subject().to_owned(),
        email,
        password,
    };
    service.close().await;
    child(url, &receipt, false, true).await;
    receipt
}

async fn after_restart(url: &str, receipt: &Receipt) {
    child(url, receipt, true, true).await;
    child(url, receipt, false, false).await;
}

#[cfg(feature = "api-tokens-sqlite")]
#[tokio::test]
async fn sqlite_api_token_process_restart() {
    let directory = tempfile::tempdir().unwrap();
    let url = sqlite_url(&directory.path().join("restart.db"));
    let receipt = exercise(&url).await;
    after_restart(&url, &receipt).await;
}

#[cfg(feature = "api-tokens-postgres")]
#[tokio::test]
#[ignore = "requires wrapper-owned PostgreSQL and an explicit service restart"]
async fn postgres_api_token_process_restart() {
    let url = std::env::var("RULLST_RECOVERY_TEST_POSTGRES_URL").unwrap();
    assert_eq!(
        url::Url::parse(&url).unwrap().path(),
        "/rullst_recovery_contract"
    );
    let path = std::env::var("RULLST_API_TOKEN_RESTART_RECEIPT").unwrap();
    match std::env::var("RULLST_SESSION_RESTART_PHASE")
        .unwrap()
        .as_str()
    {
        "exercise" => {
            let receipt = exercise(&url).await;
            std::fs::write(&path, serde_json::to_vec(&receipt).unwrap()).unwrap();
        }
        "restart" => {
            let receipt = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            after_restart(&url, &receipt).await;
        }
        _ => panic!("wrapper phase missing"),
    }
}
