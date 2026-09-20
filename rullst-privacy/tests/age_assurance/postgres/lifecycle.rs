use super::*;
use tokio::sync::Barrier;

pub(super) async fn bootstrap_and_runtime_role(url: &str, database: &mut PgConnection) {
    reset(database).await;
    assert!(matches!(
        PostgresReplayStore::connect(url, 10).await,
        Err(AgeError::StoreConfiguration)
    ));
    let absent: bool = sqlx::query_scalar(
        "SELECT NOT EXISTS(SELECT 1 FROM pg_namespace WHERE nspname = 'rullst_age_replay')",
    )
    .fetch_one(&mut *database)
    .await
    .unwrap();
    assert!(
        absent,
        "normal connection must not create missing replay state"
    );
    for capacity in [0, 100_001] {
        assert!(matches!(
            PostgresReplayStore::initialize(url, capacity).await,
            Err(AgeError::InvalidConfiguration)
        ));
    }
    let (a, b) = tokio::join!(
        PostgresReplayStore::initialize(url, 10),
        PostgresReplayStore::initialize(url, 10)
    );
    a.unwrap().close().await;
    b.unwrap().close().await;
    for statement in [
        "CREATE ROLE rullst_age_runtime LOGIN",
        "GRANT USAGE ON SCHEMA rullst_age_replay TO rullst_age_runtime",
        "GRANT SELECT, UPDATE ON rullst_age_replay.metadata TO rullst_age_runtime",
        "GRANT SELECT, INSERT, DELETE ON rullst_age_replay.claims TO rullst_age_runtime",
    ] {
        sqlx::query(statement)
            .execute(&mut *database)
            .await
            .unwrap();
    }
    let limited_url = url.replacen("postgres://postgres@", "postgres://rullst_age_runtime@", 1);
    let runtime = PostgresReplayStore::connect(limited_url, 10).await.unwrap();
    assert!(runtime.claim(nonce(), 1100, 1000).await.unwrap());
    runtime.close().await;
    reset(database).await;
    sqlx::query("DROP ROLE rullst_age_runtime")
        .execute(database)
        .await
        .unwrap();
}

pub(super) async fn signed_proof_concurrency_and_reopen(url: &str, database: &mut PgConnection) {
    reset(database).await;
    let first = PostgresReplayStore::initialize(url, 10).await.unwrap();
    let second = PostgresReplayStore::connect(url, 10).await.unwrap();
    let challenge = Arc::new(challenge(AgeMethod::VerifiedAttribute));
    let barrier = Arc::new(Barrier::new(8));
    let mut tasks = Vec::new();
    for index in 0..8 {
        let verifier = AgeVerifier::new(
            issuer(),
            if index % 2 == 0 {
                first.clone()
            } else {
                second.clone()
            },
        )
        .unwrap();
        let challenge = challenge.clone();
        let barrier = barrier.clone();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            assess(&verifier, &challenge, &FixedClock(1001)).await
        }));
    }
    let mut allowed = 0;
    for task in tasks {
        match task.await.unwrap() {
            Ok(assessment) => {
                assert_eq!(assessment.decision(), AgeDecision::Allowed);
                allowed += 1;
            }
            Err(error) => assert_eq!(error, AgeError::Replay),
        }
    }
    assert_eq!(allowed, 1);
    first.close().await;
    second.close().await;
    let reopened = PostgresReplayStore::connect(url, 10).await.unwrap();
    let verifier = AgeVerifier::new(issuer(), reopened.clone()).unwrap();
    assert_eq!(
        assess(&verifier, &challenge, &FixedClock(1001)).await,
        Err(AgeError::Replay)
    );
    let columns: Vec<String> = sqlx::query_scalar("SELECT column_name::text FROM information_schema.columns WHERE table_schema = 'rullst_age_replay' AND table_name = 'claims' ORDER BY ordinal_position")
        .fetch_all(&mut *database).await.unwrap();
    assert_eq!(columns, ["nonce_digest", "expires_at"]);
    let raw: serde_json::Value =
        serde_json::from_slice(&challenge.request_json().unwrap()).unwrap();
    let nonce: Vec<u8> = serde_json::from_value(raw["nonce"].clone()).unwrap();
    let digest: Vec<u8> = sqlx::query_scalar("SELECT nonce_digest FROM rullst_age_replay.claims")
        .fetch_one(database)
        .await
        .unwrap();
    assert_eq!(digest.len(), 32);
    assert_ne!(digest, nonce);
    let mut hash = ring::digest::Context::new(&ring::digest::SHA256);
    hash.update(b"rullst.age-replay.v1\0");
    hash.update(&nonce);
    assert_eq!(digest, hash.finish().as_ref());
    reopened.close().await;
}

pub(super) async fn quota_expiry_and_clock(url: &str, database: &mut PgConnection) {
    reset(database).await;
    let first = PostgresReplayStore::initialize(url, 1).await.unwrap();
    let second = PostgresReplayStore::connect(url, 1).await.unwrap();
    let (a, b) = (nonce(), nonce());
    let (x, y) = tokio::join!(first.claim(a, 1100, 1000), second.claim(b, 1100, 1000));
    assert!(matches!(
        (x, y),
        (Ok(true), Err(AgeError::StoreCapacity)) | (Err(AgeError::StoreCapacity), Ok(true))
    ));
    let winner = if x.is_ok() { a } else { b };
    assert!(!second.claim(winner, 1100, 1000).await.unwrap());
    assert_eq!(
        second.claim(nonce(), 1000, 1000).await,
        Err(AgeError::InvalidChallenge)
    );
    assert_eq!(
        second.claim(nonce(), 1901, 1000).await,
        Err(AgeError::InvalidChallenge)
    );
    assert!(first.claim(nonce(), 1200, 1100).await.unwrap());
    first.close().await;
    second.close().await;
    let reopened = PostgresReplayStore::connect(url, 1).await.unwrap();
    assert_eq!(
        reopened.claim(winner, 1100, 1000).await,
        Err(AgeError::ClockRollback)
    );
    assert_eq!(
        reopened.claim(nonce(), 1200, 1100).await,
        Err(AgeError::StoreCapacity)
    );
    reopened.close().await;
}
