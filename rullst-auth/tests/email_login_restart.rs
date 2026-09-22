#![cfg(any(feature = "email-login-sqlite", feature = "email-login-postgres"))]
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[allow(dead_code, unused_imports)]
#[path = "email_login/support.rs"]
mod support;
use std::process::Stdio;
use support::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(serde::Serialize, serde::Deserialize)]
struct Receipt {
    namespace: String,
    browser: String,
    consumed: String,
    pending: String,
    session: String,
    subject: String,
    pending_subject: String,
}
#[derive(serde::Serialize, serde::Deserialize)]
struct Worker {
    url: String,
    receipt: Receipt,
    redeem: bool,
}

#[tokio::test]
#[ignore = "owned subprocess; configuration must arrive on stdin"]
async fn email_login_restart_worker() {
    let mut input = Vec::new();
    tokio::io::stdin()
        .take(16384)
        .read_to_end(&mut input)
        .await
        .unwrap();
    let worker: Worker = serde_json::from_slice(&input).unwrap();
    let receipt = worker.receipt;
    let service = EmailLoginService::connect(&worker.url, keys(), config(&receipt.namespace))
        .await
        .unwrap();
    let clock = Clock::new();
    let browser = BrowserBinding::from_cookie(&receipt.browser).unwrap();
    assert_eq!(
        service
            .accounts()
            .verify_session(&receipt.session, clock.now().unwrap())
            .await
            .unwrap(),
        Some(receipt.subject)
    );
    assert_eq!(
        service
            .redeem(&receipt.consumed, &browser, &clock)
            .await
            .unwrap_err(),
        RecoveryError::InvalidAction
    );
    if worker.redeem {
        let session = service
            .redeem(&receipt.pending, &browser, &clock)
            .await
            .unwrap();
        assert_eq!(session.subject(), receipt.pending_subject);
        assert!(
            service
                .redeem(&receipt.pending, &browser, &clock)
                .await
                .is_err()
        );
    }
    service.close().await;
}

async fn child(url: &str, receipt: &Receipt, redeem: bool) {
    let value = serde_json::json!({"url":url,"receipt":receipt,"redeem":redeem});
    let mut child = tokio::process::Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "email_login_restart_worker"])
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
        "owned restart worker failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

async fn exercise(url: &str) -> Receipt {
    let namespace = unique();
    let clock = Clock::new();
    let service = EmailLoginService::initialize(url, keys(), config(&namespace))
        .await
        .unwrap();
    let (subject, email, _) = account(&service, &clock).await;
    let browser = BrowserBinding::generate().unwrap();
    let used = issue(&service, &email, &browser, &clock).await;
    let consumed = token(&used);
    let session = service.redeem(&consumed, &browser, &clock).await.unwrap();
    let (pending_subject, email, _) = account(&service, &clock).await;
    let pending = issue(&service, &email, &browser, &clock).await;
    let receipt = Receipt {
        namespace,
        browser: browser.expose_cookie().to_owned(),
        consumed,
        pending: token(&pending),
        session: session.token().expose().to_owned(),
        subject,
        pending_subject,
    };
    service.complete_notice(&pending, &clock).await.unwrap();
    service.close().await;
    child(url, &receipt, false).await;
    receipt
}

#[cfg(feature = "email-login-sqlite")]
#[tokio::test]
async fn sqlite_email_login_process_restart() {
    let directory = tempfile::tempdir().unwrap();
    let url = sqlite_url(&directory.path().join("restart.db"));
    let receipt = exercise(&url).await;
    child(&url, &receipt, true).await;
}

#[cfg(feature = "email-login-postgres")]
#[tokio::test]
#[ignore = "requires wrapper-owned PostgreSQL and an explicit service restart"]
async fn postgres_email_login_process_restart() {
    let url = std::env::var("RULLST_RECOVERY_TEST_POSTGRES_URL").unwrap();
    assert_eq!(
        url::Url::parse(&url).unwrap().path(),
        "/rullst_recovery_contract"
    );
    let path = std::env::var("RULLST_EMAIL_LOGIN_RESTART_RECEIPT").unwrap();
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
            child(&url, &receipt, true).await;
        }
        _ => panic!("wrapper phase missing"),
    }
}
