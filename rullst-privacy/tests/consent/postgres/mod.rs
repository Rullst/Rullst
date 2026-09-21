use super::*;
use sqlx::{Connection, PgConnection};
use std::{process::Stdio, time::Duration};

mod failures;
mod lifecycle;

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

async fn reset(database: &mut PgConnection) {
    // fixture_url admits only the owned disposable database, never user state.
    sqlx::query("DROP SCHEMA IF EXISTS rullst_consent CASCADE")
        .execute(database)
        .await
        .unwrap();
}

async fn grant(
    gate: &ConsentGate<PostgresConsentStore>,
    revision: u64,
    now: i64,
    expiry: i64,
) -> Result<ConsentRecord, ConsentError> {
    gate.choose_with_clock(
        &subject(),
        &purpose(),
        &submission(revision, ConsentChoice::Granted),
        expiry,
        &Clock::at(now),
    )
    .await
}

#[tokio::test]
async fn invalid_postgres_configuration_is_rejected_without_network_or_secret_errors() {
    for url in [
        "",
        "mock_local",
        "https://db.invalid/secret",
        "postgres://localhost/db?passwrod=secret",
        "postgres://localhost/db?sslmode=disable&sslmode=prefer",
        "postgres://localhost/db#secret",
    ] {
        assert!(matches!(
            PostgresConsentStore::connect(url, 10).await,
            Err(ConsentError::InvalidConfiguration)
        ));
    }
    for capacity in [0, 100_001, usize::MAX] {
        assert!(matches!(
            PostgresConsentStore::connect("postgres://localhost/secret", capacity).await,
            Err(ConsentError::InvalidConfiguration)
        ));
    }
    assert!(!format!("{:?}", ConsentError::StoreUnavailable).contains("secret"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires the owned PostgreSQL wrapper; missing acceptance must not silently pass"]
async fn shared_consent_contract() {
    let url = fixture_url();
    if std::env::var("RULLST_PRIVACY_POSTGRES_PHASE").as_deref() == Ok("restart") {
        verify_retained_withdrawal(&url).await;
        println!("PostgreSQL preserved consent withdrawal, revision and clock across restart");
        return;
    }
    let mut database = PgConnection::connect(&url).await.unwrap();
    lifecycle::bootstrap_and_runtime_role(&url, &mut database).await;
    lifecycle::concurrent_grants_and_withdrawal(&url, &mut database).await;
    lifecycle::scope_expiry_quota_and_reopen(&url, &mut database).await;
    failures::schema_and_durability(&url, &mut database).await;
    failures::failure_cancellation_and_clock_wait(&url, &mut database).await;
    reset(&mut database).await;
    let store = PostgresConsentStore::initialize(&url, 10).await.unwrap();
    let gate = ConsentGate::new(store.clone()).unwrap();
    grant(&gate, 0, 3000, 3100).await.unwrap();
    gate.withdraw_with_clock(&subject(), &purpose(), &Clock::at(3000))
        .await
        .unwrap();
    store.close().await;
    database.close().await.unwrap();
    fresh_process().await;
    println!(
        "PostgreSQL consent bootstrap, concurrency, withdrawal, quota, clock, cancellation and process contracts passed"
    );
}

async fn verify_retained_withdrawal(url: &str) {
    let store = PostgresConsentStore::connect(url, 10).await.unwrap();
    let gate = ConsentGate::new(store.clone()).unwrap();
    let retained = gate
        .current_with_clock(&subject(), &purpose(), &Clock::at(3000))
        .await
        .unwrap();
    assert_eq!(retained.choice(), ConsentChoice::Withdrawn);
    assert_eq!(retained.revision(), 2);
    assert_eq!(
        grant(&gate, 1, 3000, 3100).await,
        Err(ConsentError::RevisionConflict)
    );
    assert_eq!(
        gate.allows_with_clock(&subject(), &purpose(), &Clock::at(2999))
            .await,
        Err(ConsentError::ClockRollback)
    );
    store.close().await;
}

#[tokio::test]
#[ignore = "child process invoked by the owned PostgreSQL contract"]
async fn consent_child() {
    verify_retained_withdrawal(&fixture_url()).await;
    println!("fresh PostgreSQL process observed consent withdrawal");
}

async fn fresh_process() {
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "postgres::consent_child",
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
            panic!("fresh consent process exceeded its deadline");
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
            .contains("fresh PostgreSQL process observed consent withdrawal")
    );
}
