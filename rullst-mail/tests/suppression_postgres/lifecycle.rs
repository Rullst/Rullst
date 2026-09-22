use super::support::*;

pub async fn run(url: &str) {
    let namespace = unique();
    let (first, second) = tokio::join!(
        PostgresSuppressionStore::initialize(url, key(), config(&namespace)),
        PostgresSuppressionStore::initialize(url, key(), config(&namespace))
    );
    let first = first.unwrap();
    let second = second.unwrap();
    let recipient = "Member@Example.COM";
    let time = now() - 60;
    let initial = event("original", recipient, SuppressionReason::HardBounce, time);
    let (a, b) = tokio::join!(
        first.record(initial.clone()),
        second.record(initial.clone())
    );
    assert_eq!(a.unwrap(), b.unwrap());
    let record = second.lookup(recipient).await.unwrap().unwrap();
    assert_eq!(record.recipient(), "Member@example.com");
    assert_eq!(record.reason(), SuppressionReason::HardBounce);
    assert!(second.lookup("member@example.com").await.unwrap().is_none());
    for conflict in [
        event(
            "original",
            "other@example.com",
            SuppressionReason::HardBounce,
            time,
        ),
        event("original", recipient, SuppressionReason::Manual, time),
        event(
            "original",
            recipient,
            SuppressionReason::HardBounce,
            time + 1,
        ),
    ] {
        assert_eq!(
            first.record(conflict).await.unwrap_err(),
            SuppressionError::EventConflict
        );
    }
    let older = event(
        "complaint",
        recipient,
        SuppressionReason::SpamComplaint,
        time - 1,
    );
    let newer = event("manual", recipient, SuppressionReason::Manual, time + 1);
    let (a, b) = tokio::join!(first.record(older), second.record(newer));
    a.unwrap();
    b.unwrap();
    let record = first.lookup(recipient).await.unwrap().unwrap();
    assert_eq!(record.reason(), SuppressionReason::SpamComplaint);
    assert_eq!(record.first_seen_at(), time - 1);
    assert_eq!(record.last_seen_at(), time + 1);
    assert_eq!(first.snapshot().await.unwrap().recipients(), 1);
    assert_eq!(first.snapshot().await.unwrap().events(), 3);
    assert_eq!(first.prune_events_before(time).await.unwrap(), 1);
    assert_eq!(first.snapshot().await.unwrap().events(), 2);
    assert_eq!(
        first.lookup(recipient).await.unwrap().unwrap().reason(),
        SuppressionReason::SpamComplaint
    );
    assert!(first.prune_events_before(0).await.is_err());
    assert!(first.prune_events_before(now() + 60).await.is_err());
    let other = PostgresSuppressionStore::initialize(url, key(), config(&unique()))
        .await
        .unwrap();
    assert!(other.lookup(recipient).await.unwrap().is_none());
    // The installed guard checks the selected namespace on every real dispatch.
    let (driver, inbox) = MemoryDriver::isolated();
    let guard = SuppressionGuard::new(driver, second.clone());
    assert!(matches!(
        guard.send(&message(recipient)).await,
        Err(MailError::SuppressedRecipient { .. })
    ));
    assert!(matches!(
        guard.send_for_tenant("school-a", &message(recipient)).await,
        Err(MailError::SuppressedRecipient { .. })
    ));
    assert!(matches!(
        guard
            .send_with_delivery_id(&message(recipient), "stable-delivery-fixture")
            .await,
        Err(MailError::SuppressedRecipient { .. })
    ));
    assert!(inbox.lock().unwrap().is_empty());
    guard.send(&message("allowed@example.com")).await.unwrap();
    assert_eq!(inbox.lock().unwrap().len(), 1);
    first
        .record(event(
            "suppress-later",
            "allowed@example.com",
            SuppressionReason::Manual,
            time,
        ))
        .await
        .unwrap();
    assert!(guard.send(&message("allowed@example.com")).await.is_err());
    assert_eq!(inbox.lock().unwrap().len(), 1);
    let (driver, other_inbox) = MemoryDriver::isolated();
    let other_guard = SuppressionGuard::new(driver, other.clone());
    other_guard.send(&message(recipient)).await.unwrap();
    assert_eq!(other_inbox.lock().unwrap().len(), 1);
    let raw = sqlx::PgPool::connect(url).await.unwrap();
    let recipients: Vec<String>=sqlx::query_scalar("SELECT row_to_json(r)::text FROM rullst_mail_pg_suppression_recipients r WHERE namespace = $1").bind(&namespace).fetch_all(&raw).await.unwrap();
    let events: Vec<String> = sqlx::query_scalar(
        "SELECT row_to_json(e)::text FROM rullst_mail_pg_suppression_events e WHERE namespace = $1",
    )
    .bind(&namespace)
    .fetch_all(&raw)
    .await
    .unwrap();
    for stored in recipients.iter().chain(events.iter()) {
        assert!(!stored.contains("example.com"));
        assert!(!stored.contains("original"));
    }
    raw.close().await;
    first.close().await;
    second.close().await;
    other.close().await;
    assert!(matches!(
        guard.send(&message("new@example.com")).await,
        Err(MailError::SuppressionUnavailable)
    ));
    assert_eq!(inbox.lock().unwrap().len(), 1);
    let reopened = PostgresSuppressionStore::connect(url, key(), config(&namespace))
        .await
        .unwrap();
    assert!(reopened.lookup(recipient).await.unwrap().is_some());
    assert!(
        PostgresSuppressionStore::connect(
            url,
            key(),
            PostgresSuppressionConfig::new(&namespace, 99, 100).unwrap()
        )
        .await
        .is_err()
    );
    let wrong_key = SuppressionKey::new(std::array::from_fn(|i| (i + 1) as u8)).unwrap();
    assert!(
        PostgresSuppressionStore::connect(url, wrong_key, config(&namespace))
            .await
            .is_err()
    );
    assert!(
        PostgresSuppressionStore::connect(url, key(), config(&unique()))
            .await
            .is_err()
    );
    reopened.close().await;
}
