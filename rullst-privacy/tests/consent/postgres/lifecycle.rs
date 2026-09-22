use super::*;
use tokio::sync::Barrier;

pub(super) async fn bootstrap_and_runtime_role(url: &str, database: &mut PgConnection) {
    reset(database).await;
    assert!(matches!(
        PostgresConsentStore::connect(url, 10).await,
        Err(ConsentError::StoreConfiguration)
    ));
    let absent: bool = sqlx::query_scalar(
        "SELECT NOT EXISTS(SELECT 1 FROM pg_namespace WHERE nspname = 'rullst_consent')",
    )
    .fetch_one(&mut *database)
    .await
    .unwrap();
    assert!(absent);
    let (first, second) = tokio::join!(
        PostgresConsentStore::initialize(url, 10),
        PostgresConsentStore::initialize(url, 10)
    );
    first.unwrap().close().await;
    second.unwrap().close().await;
    assert!(matches!(
        PostgresConsentStore::initialize(url, 11).await,
        Err(ConsentError::StoreConfiguration)
    ));
    for statement in [
        "CREATE ROLE rullst_consent_runtime LOGIN",
        "GRANT USAGE ON SCHEMA rullst_consent TO rullst_consent_runtime",
        "GRANT SELECT, UPDATE ON rullst_consent.metadata TO rullst_consent_runtime",
        "GRANT SELECT, INSERT, UPDATE ON rullst_consent.choices TO rullst_consent_runtime",
    ] {
        sqlx::query(statement)
            .execute(&mut *database)
            .await
            .unwrap();
    }
    let limited_url = url.replacen(
        "postgres://postgres@",
        "postgres://rullst_consent_runtime@",
        1,
    );
    let runtime = PostgresConsentStore::connect(limited_url, 10)
        .await
        .unwrap();
    let gate = ConsentGate::new(runtime.clone()).unwrap();
    grant(&gate, 0, 1000, 1100).await.unwrap();
    gate.withdraw_with_clock(&subject(), &purpose(), &Clock::at(1000))
        .await
        .unwrap();
    assert!(
        !gate
            .allows_with_clock(&subject(), &purpose(), &Clock::at(1000))
            .await
            .unwrap()
    );
    runtime.close().await;
    reset(database).await;
    sqlx::query("DROP ROLE rullst_consent_runtime")
        .execute(database)
        .await
        .unwrap();
}

pub(super) async fn concurrent_grants_and_withdrawal(url: &str, database: &mut PgConnection) {
    reset(database).await;
    let first = PostgresConsentStore::initialize(url, 10).await.unwrap();
    let second = PostgresConsentStore::connect(url, 10).await.unwrap();
    let barrier = Arc::new(Barrier::new(12));
    let mut tasks = Vec::new();
    for index in 0..12 {
        let gate = ConsentGate::new(if index % 2 == 0 {
            first.clone()
        } else {
            second.clone()
        })
        .unwrap();
        let barrier = barrier.clone();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            grant(&gate, 0, 1000, 1100).await
        }));
    }
    let mut accepted = 0;
    for task in tasks {
        match task.await.unwrap() {
            Ok(record) => {
                assert_eq!(record.revision(), 1);
                accepted += 1;
            }
            Err(error) => assert_eq!(error, ConsentError::RevisionConflict),
        }
    }
    assert_eq!(accepted, 1);
    let a = ConsentGate::new(first.clone()).unwrap();
    let b = ConsentGate::new(second.clone()).unwrap();
    let clock = Clock::at(1000);
    let subject = subject();
    let purpose = purpose();
    let (withdrawal, stale) = tokio::join!(
        a.withdraw_with_clock(&subject, &purpose, &clock),
        grant(&b, 1, 1000, 1100)
    );
    withdrawal.unwrap();
    assert!(stale.is_ok() || stale == Err(ConsentError::RevisionConflict));
    assert!(
        !b.allows_with_clock(&subject, &purpose, &clock)
            .await
            .unwrap()
    );
    assert_eq!(
        grant(&b, 1, 1000, 1100).await,
        Err(ConsentError::RevisionConflict)
    );
    first.close().await;
    second.close().await;
}

pub(super) async fn scope_expiry_quota_and_reopen(url: &str, database: &mut PgConnection) {
    reset(database).await;
    let first = PostgresConsentStore::initialize(url, 1).await.unwrap();
    let second = PostgresConsentStore::connect(url, 1).await.unwrap();
    let a = ConsentGate::new(first.clone()).unwrap();
    let b = ConsentGate::new(second.clone()).unwrap();
    grant(&a, 0, 1000, 1100).await.unwrap();
    for other in [
        ConsentSubject::new("other", "school-1").unwrap(),
        ConsentSubject::new("user-1", "other-school").unwrap(),
    ] {
        assert!(
            !b.allows_with_clock(&other, &purpose(), &Clock::at(1000))
                .await
                .unwrap()
        );
        assert_eq!(
            b.withdraw_with_clock(&other, &purpose(), &Clock::at(1000))
                .await,
            Err(ConsentError::StoreCapacity)
        );
    }
    for other in [
        ConsentPurpose::new("other-purpose", "notice-v1").unwrap(),
        ConsentPurpose::new("optional-digest", "notice-v2").unwrap(),
    ] {
        assert!(
            !b.allows_with_clock(&subject(), &other, &Clock::at(1000))
                .await
                .unwrap()
        );
    }
    assert!(
        !b.allows_with_clock(&subject(), &purpose(), &Clock::at(1100))
            .await
            .unwrap()
    );
    first.close().await;
    second.close().await;
    let reopened = PostgresConsentStore::connect(url, 1).await.unwrap();
    let gate = ConsentGate::new(reopened.clone()).unwrap();
    assert_eq!(
        gate.allows_with_clock(&subject(), &purpose(), &Clock::at(1099))
            .await,
        Err(ConsentError::ClockRollback)
    );
    assert_eq!(
        gate.withdraw_with_clock(
            &ConsentSubject::new("other", "school-1").unwrap(),
            &purpose(),
            &Clock::at(1200)
        )
        .await,
        Err(ConsentError::StoreCapacity)
    );
    gate.withdraw_with_clock(&subject(), &purpose(), &Clock::at(1200))
        .await
        .unwrap();
    let columns: Vec<String> = sqlx::query_scalar("SELECT column_name::text FROM information_schema.columns WHERE table_schema = 'rullst_consent' AND table_name = 'choices' ORDER BY ordinal_position")
        .fetch_all(&mut *database).await.unwrap();
    assert_eq!(
        columns,
        [
            "scope_digest",
            "revision",
            "policy_version",
            "choice",
            "changed_at",
            "valid_until"
        ]
    );
    let (digest, choice, revision): (Vec<u8>, i64, i64) =
        sqlx::query_as("SELECT scope_digest,choice,revision FROM rullst_consent.choices")
            .fetch_one(database)
            .await
            .unwrap();
    assert_eq!(digest.len(), 32);
    assert_eq!(choice, 3);
    assert_eq!(revision, 2);
    reopened.close().await;
}
