use super::{server::Receiver, support::*};

#[tokio::test]
async fn cancellation_after_send_preserves_uncertain_acceptance_and_never_revives() {
    let clock = ManualClock::new();
    let receiver = Receiver::start(clock.clone(), false).await;
    let (path, url) = fixture();
    let namespace = unique();
    let store = open(&url, &namespace, receiver.destination(), &clock).await;
    let operator = open(&url, &namespace, receiver.destination(), &clock).await;
    let event = store
        .enqueue("cancel", "ready", b"{}".to_vec())
        .await
        .unwrap();
    receiver.pause.store(true, Ordering::SeqCst);
    let worker = tokio::spawn(async move {
        let result = store.dispatch_next("one").await;
        store.close().await;
        result
    });
    tokio::time::timeout(Duration::from_secs(5), receiver.received.notified())
        .await
        .unwrap();
    operator.cancel(event.id().as_str()).await.unwrap();
    receiver.release.notify_one();
    assert_eq!(
        worker.await.unwrap(),
        Err(WebhookError::Acknowledgement {
            delivery_id: event.id().as_str().to_owned(),
            status: 200
        })
    );
    assert_eq!(receiver.effects.lock().await.len(), 1);
    assert!(operator.retry_failed(event.id().as_str()).await.is_err());
    assert_eq!(
        operator.dispatch_next("two").await.unwrap(),
        WebhookDispatch::Idle
    );
    operator.close().await;
    cleanup(&path);
}

#[tokio::test]
async fn competing_instances_retry_budgets_deadlines_and_offline_production_guard() {
    let clock = ManualClock::new();
    let receiver = Receiver::start(clock.clone(), false).await;
    let (path, url) = fixture();
    let namespace = unique();
    let first = open(&url, &namespace, receiver.destination(), &clock).await;
    let second = open(&url, &namespace, receiver.destination(), &clock).await;
    receiver.status.store(429, Ordering::SeqCst);
    let event = first
        .enqueue("retry", "ready", b"{}".to_vec())
        .await
        .unwrap();
    let (one, two) = tokio::join!(first.dispatch_next("one"), second.dispatch_next("two"));
    let outcomes = [one.unwrap(), two.unwrap()];
    assert_eq!(
        outcomes
            .iter()
            .filter(|r| **r == WebhookDispatch::Idle)
            .count(),
        1
    );
    let scheduled = outcomes
        .iter()
        .find_map(|r| {
            if let WebhookDispatch::RetryScheduled {
                available_at_ms, ..
            } = r
            {
                Some(*available_at_ms)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(scheduled, clock.now_millis().unwrap() + 3000);
    clock.advance(600_000);
    for attempt in 2..=10 {
        let outcome = first.dispatch_next("one").await.unwrap();
        if attempt == 10 {
            assert!(matches!(
                outcome,
                WebhookDispatch::DeadLettered { attempt: 10, .. }
            ));
        } else {
            assert!(matches!(outcome, WebhookDispatch::RetryScheduled { .. }));
        }
        clock.advance(600_000);
    }
    assert_eq!(receiver.requests.lock().await.len(), 10);
    assert_eq!(first.failed(10).await.unwrap()[0].attempts(), 10);
    first.retry_failed(event.id().as_str()).await.unwrap();
    receiver.status.store(200, Ordering::SeqCst);
    assert!(matches!(
        second.dispatch_next("two").await.unwrap(),
        WebhookDispatch::Accepted { .. }
    ));
    assert!(first.retry_failed(event.id().as_str()).await.is_err());
    first
        .enqueue("expire", "ready", b"{}".to_vec())
        .await
        .unwrap();
    clock.advance(86_400_001);
    assert!(matches!(
        first.dispatch_next("one").await.unwrap(),
        WebhookDispatch::DeadLettered { .. }
    ));
    assert_eq!(receiver.requests.lock().await.len(), 11);
    let production = config(
        &unique(),
        WebhookDestination::approved_https("https://receiver.example/events").unwrap(),
    )
    .require_production()
    .unwrap();
    assert!(
        WebhookOutbox::open(
            &url,
            production,
            WebhookSigningKey::new("fixture", "").unwrap(),
            storage(),
            clock.clone()
        )
        .await
        .is_err()
    );
    assert!(
        config(&unique(), receiver.destination())
            .require_production()
            .is_err()
    );
    let offline = WebhookOutbox::open(
        &url,
        config(&unique(), receiver.destination()),
        WebhookSigningKey::new("fixture", "mock_offline").unwrap(),
        storage(),
        clock.clone(),
    )
    .await
    .unwrap();
    offline
        .enqueue("offline", "ready", b"{}".to_vec())
        .await
        .unwrap();
    assert!(matches!(
        offline.dispatch_next("offline").await.unwrap(),
        WebhookDispatch::Accepted { offline: true, .. }
    ));
    assert_eq!(receiver.requests.lock().await.len(), 11);
    offline.close().await;
    first.close().await;
    second.close().await;
    cleanup(&path);
}

#[tokio::test]
async fn private_destination_storage_outage_and_cancelled_sql_never_dispatch() {
    let clock = ManualClock::new();
    let receiver = Receiver::start(clock.clone(), false).await;
    let (path, url) = fixture();
    let namespace = unique();
    let store = open(&url, &namespace, receiver.destination(), &clock).await;
    store.enqueue("one", "ready", b"{}".to_vec()).await.unwrap();
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .unwrap();
    let lock = pool.begin_with("BEGIN IMMEDIATE").await.unwrap();
    let clone = store.clone();
    let task = tokio::spawn(async move { clone.dispatch_next("blocked").await });
    tokio::time::sleep(Duration::from_millis(100)).await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    lock.rollback().await.unwrap();
    assert!(receiver.requests.lock().await.is_empty());
    let private = WebhookDestination::approved_https("https://127.0.0.1:443/events").unwrap();
    let denied = open(&url, &unique(), private, &clock).await;
    denied
        .enqueue("private", "ready", b"{}".to_vec())
        .await
        .unwrap();
    assert!(matches!(
        denied.dispatch_next("one").await.unwrap(),
        WebhookDispatch::DeadLettered { .. }
    ));
    assert_eq!(
        denied.failed(1).await.unwrap()[0].failure_code(),
        "webhook.destination_denied"
    );
    // A real schema outage cannot use memory or send unclaimed work.
    sqlx::query("ALTER TABLE rullst_messaging_deliveries RENAME TO fixture_missing_deliveries")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(store.dispatch_next("one").await, Err(WebhookError::Storage));
    assert!(receiver.requests.lock().await.is_empty());
    sqlx::query("ALTER TABLE fixture_missing_deliveries RENAME TO rullst_messaging_deliveries")
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        store.dispatch_next("one").await.unwrap(),
        WebhookDispatch::Accepted { .. }
    ));
    store.close().await;
    denied.close().await;
    pool.close().await;
    cleanup(&path);
}

#[tokio::test]
async fn namespace_quota_and_signing_identity_are_durable_and_fail_closed() {
    let clock = ManualClock::new();
    let receiver = Receiver::start(clock.clone(), false).await;
    let (path, url) = fixture();
    let namespace = unique();
    let policy = WebhookConfig::new(&namespace, receiver.destination(), 1).unwrap();
    let store = WebhookOutbox::open(&url, policy.clone(), key(), storage(), clock.clone())
        .await
        .unwrap();
    let first = store.enqueue("one", "ready", b"{}".to_vec()).await.unwrap();
    assert_eq!(
        store.enqueue("two", "ready", b"{}".to_vec()).await,
        Err(WebhookError::Capacity)
    );
    assert!(
        store
            .enqueue("one", "ready", b"{}".to_vec())
            .await
            .unwrap()
            .is_duplicate()
    );
    assert!(
        WebhookOutbox::open(
            &url,
            policy.clone(),
            WebhookSigningKey::new("changed", "mock_other").unwrap(),
            storage(),
            clock.clone()
        )
        .await
        .is_err()
    );
    let other = open(&url, &unique(), receiver.destination(), &clock).await;
    assert!(other.cancel(first.id().as_str()).await.is_err());
    assert!(other.retry_failed(first.id().as_str()).await.is_err());
    assert!(other.failed(10).await.unwrap().is_empty());
    let reopened = WebhookOutbox::open(&url, policy, key(), storage(), clock.clone())
        .await
        .unwrap();
    assert!(matches!(
        reopened.dispatch_next("reopen").await.unwrap(),
        WebhookDispatch::Accepted { .. }
    ));
    assert_eq!(
        store.enqueue("two", "ready", b"{}".to_vec()).await,
        Err(WebhookError::Capacity)
    );
    clock.advance(86_400_001);
    assert_eq!(
        store
            .purge_terminal(clock.now_millis().unwrap() - 86_400_000, 10)
            .await
            .unwrap(),
        1
    );
    store.enqueue("two", "ready", b"{}".to_vec()).await.unwrap();
    store.close().await;
    other.close().await;
    reopened.close().await;
    cleanup(&path);
}
