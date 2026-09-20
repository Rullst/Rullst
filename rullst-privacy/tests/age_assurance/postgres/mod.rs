use super::*;
use ring::rand::{SystemRandom, generate};
use sqlx::{Connection, PgConnection};
use std::{process::Stdio, time::Duration};

mod clock_wait;
mod failures;
mod lifecycle;

fn nonce() -> [u8; 32] {
    generate(&SystemRandom::new()).unwrap().expose()
}

fn fixture_url() -> String {
    assert_eq!(
        std::env::var("RULLST_PRIVACY_POSTGRES_DISPOSABLE").as_deref(),
        Ok("1")
    );
    let url = std::env::var("RULLST_PRIVACY_TEST_POSTGRES_URL").unwrap();
    assert!(url.starts_with("postgres://postgres@127.0.0.1:"));
    assert!(url.ends_with("/rullst_privacy_contract"));
    url
}

fn restart_nonce() -> [u8; 32] {
    let hex = std::env::var("RULLST_PRIVACY_RESTART_NONCE").unwrap();
    assert_eq!(hex.len(), 64);
    std::array::from_fn(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap())
}

async fn reset(database: &mut PgConnection) {
    // The wrapper creates an owned disposable database, and fixture_url rejects
    // arbitrary application endpoints. No application data is a valid target.
    sqlx::query("DROP SCHEMA IF EXISTS rullst_age_replay CASCADE")
        .execute(database)
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires the owned PostgreSQL wrapper; never silently skips missing database acceptance"]
async fn shared_replay_contract() {
    let url = fixture_url();
    if std::env::var("RULLST_PRIVACY_POSTGRES_PHASE").as_deref() == Ok("restart") {
        let store = PostgresReplayStore::connect(&url, 10).await.unwrap();
        assert!(!store.claim(restart_nonce(), 3100, 3000).await.unwrap());
        assert_eq!(
            store.claim(nonce(), 3100, 2999).await,
            Err(AgeError::ClockRollback)
        );
        store.close().await;
        println!("PostgreSQL preserved consumed nonce and clock across server restart");
        return;
    }
    let mut database = PgConnection::connect(&url).await.unwrap();
    lifecycle::bootstrap_and_runtime_role(&url, &mut database).await;
    lifecycle::signed_proof_concurrency_and_reopen(&url, &mut database).await;
    lifecycle::quota_expiry_and_clock(&url, &mut database).await;
    clock_wait::recheck_after_locked_storage(&url, &mut database).await;
    failures::schema_and_metadata_drift(&url, &mut database).await;
    failures::atomic_failure_and_cancellation(&url, &mut database).await;
    failures::durability_settings(&url, &mut database).await;
    reset(&mut database).await;
    let store = PostgresReplayStore::initialize(&url, 10).await.unwrap();
    assert!(store.claim(restart_nonce(), 3100, 3000).await.unwrap());
    store.close().await;
    fresh_process_replay().await;
    database.close().await.unwrap();
    println!(
        "PostgreSQL bootstrap, multi-pool concurrency, quota, schema, failure, cancellation and process contracts passed"
    );
}

#[tokio::test]
#[ignore = "child process invoked by the owned PostgreSQL contract"]
async fn replay_child() {
    let url = fixture_url();
    let store = PostgresReplayStore::connect(&url, 10).await.unwrap();
    assert!(!store.claim(restart_nonce(), 3100, 3000).await.unwrap());
    store.close().await;
    println!("fresh PostgreSQL client observed consumed nonce");
}

async fn fresh_process_replay() {
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "postgres::replay_child",
            "--exact",
            "--ignored",
            "--nocapture",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let started = std::time::Instant::now();
    while child.try_wait().unwrap().is_none() {
        if started.elapsed() > Duration::from_secs(20) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("fresh PostgreSQL client exceeded its deadline");
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("fresh PostgreSQL client observed consumed nonce")
    );
}
