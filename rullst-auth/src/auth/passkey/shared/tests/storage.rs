use super::*;

pub(super) async fn quotas_expiry_configuration_and_corruption(url: &str) {
    let (raw, a, b, clock) = reset(url).await;
    assert!(
        Store::connect_with_clock(
            url,
            CeremonyStoreConfig::new("wrong-epoch", 8, 30).unwrap(),
            clock.clone()
        )
        .await
        .is_err()
    );
    assert!(
        Store::initialize_with_clock(
            url,
            CeremonyStoreConfig::new("test-epoch", 9, 30).unwrap(),
            clock.clone()
        )
        .await
        .is_err()
    );
    for id in 0..8 {
        a.issue(&intent(id)).await.unwrap();
    }
    assert!(matches!(b.issue(&intent(9)).await, Err(Error::Capacity)));
    assert!(matches!(
        b.consume([0; 32], [8; 32], CeremonyKind::Registration)
            .await,
        Err(Error::Rejected)
    ));
    assert!(matches!(
        b.consume([0; 32], [7; 32], CeremonyKind::Authentication)
            .await,
        Err(Error::Rejected)
    ));
    let consumed = b
        .consume([0; 32], [7; 32], CeremonyKind::Registration)
        .await
        .unwrap();
    clock.set(1029);
    a.confirm(&consumed).await.unwrap();
    clock.set(1030);
    assert!(matches!(a.confirm(&consumed).await, Err(Error::Expired)));
    a.issue(&intent(9)).await.unwrap(); // bounded cleanup makes room only for expired entries
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rullst_passkey_ceremony.pending")
        .fetch_one(&raw)
        .await
        .unwrap();
    assert_eq!(count, 1);
    clock.set(1029);
    assert!(matches!(b.issue(&intent(10)).await, Err(Error::Corrupt)));
    clock.set(1030);
    sqlx::query("UPDATE rullst_passkey_ceremony.pending SET credentials=$1")
        .bind(vec![1u8])
        .execute(&raw)
        .await
        .unwrap();
    assert!(matches!(
        b.consume([9; 32], [7; 32], CeremonyKind::Registration)
            .await,
        Err(Error::Corrupt)
    ));
    sqlx::query(
        "ALTER TABLE rullst_passkey_ceremony.pending DROP CONSTRAINT pending_credentials_check",
    )
    .execute(&raw)
    .await
    .unwrap();
    sqlx::query("UPDATE rullst_passkey_ceremony.pending SET credentials=$1")
        .bind(vec![1u8; 1025])
        .execute(&raw)
        .await
        .unwrap();
    assert!(matches!(
        b.consume([9; 32], [7; 32], CeremonyKind::Registration)
            .await,
        Err(Error::Corrupt)
    ));
    sqlx::query("DELETE FROM rullst_passkey_ceremony.metadata")
        .execute(&raw)
        .await
        .unwrap();
    assert!(matches!(a.issue(&intent(12)).await, Err(Error::Corrupt)));
    assert!(
        Store::connect_with_clock(url, config(), clock.clone())
            .await
            .is_err()
    );
    assert!(
        Store::initialize_with_clock(url, config(), clock)
            .await
            .is_err(),
        "initialize must not repair missing state"
    );
    a.close().await;
    b.close().await;
    raw.close().await;
}

async fn wait_for_metadata_lock(raw: &sqlx::PgPool) {
    for _ in 0..100 {
        let blocked: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_stat_activity WHERE wait_event_type='Lock' AND query LIKE '%rullst_passkey_ceremony.metadata%' AND pid<>pg_catalog.pg_backend_pid())").fetch_one(raw).await.unwrap();
        if blocked {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!("owned ceremony operation did not reach the metadata lock");
}

pub(super) async fn lock_wait_expiry_cancellation_and_process(url: &str) {
    let (raw, a, b, clock) = reset(url).await;
    a.issue(&intent(77)).await.unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "auth::passkey::shared::tests::process_completion",
        ])
        .env("RULLST_PASSKEY_CHILD", "consume")
        .env("RULLST_PASSKEY_TEST_POSTGRES_URL", url)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(matches!(
        b.consume([77; 32], [7; 32], CeremonyKind::Registration)
            .await,
        Err(Error::Rejected)
    ));
    a.issue(&intent(1)).await.unwrap();
    let mut held = raw.begin().await.unwrap();
    sqlx::query("SELECT id FROM rullst_passkey_ceremony.metadata WHERE id=1 FOR UPDATE")
        .execute(&mut *held)
        .await
        .unwrap();
    let other = b.clone();
    let task = tokio::spawn(async move {
        other
            .consume([1; 32], [7; 32], CeremonyKind::Registration)
            .await
    });
    wait_for_metadata_lock(&raw).await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    held.commit().await.unwrap();
    b.consume([1; 32], [7; 32], CeremonyKind::Registration)
        .await
        .unwrap();
    a.issue(&intent(2)).await.unwrap();
    let mut held = raw.begin().await.unwrap();
    sqlx::query("SELECT id FROM rullst_passkey_ceremony.metadata WHERE id=1 FOR UPDATE")
        .execute(&mut *held)
        .await
        .unwrap();
    let other = b.clone();
    let task = tokio::spawn(async move {
        other
            .consume([2; 32], [7; 32], CeremonyKind::Registration)
            .await
    });
    wait_for_metadata_lock(&raw).await;
    clock.set(1030);
    held.commit().await.unwrap();
    assert!(matches!(task.await.unwrap(), Err(Error::Expired)));
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM rullst_passkey_ceremony.pending WHERE challenge=$1",
    )
    .bind([2u8; 32].as_slice())
    .fetch_one(&raw)
    .await
    .unwrap();
    assert_eq!(count, 1, "lock-wait expiry must not delete pending state");
    a.close().await;
    b.close().await;
    assert!(matches!(a.issue(&intent(9)).await, Err(Error::Unavailable)));
    raw.close().await;
}
