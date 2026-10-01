use super::support::*;

fn storage_key(id: &str, byte: u8) -> MessagingStorageKey {
    MessagingStorageKey::try_new(id, [byte; 32]).unwrap()
}

/// The configuration binding follows the primary key, so the key that sealed
/// it at initialization can leave the bounded keyring.
pub async fn run(url: &str) {
    let namespace = unique();
    let clock = ManualClock::new();
    let initial = PostgresRecurringStore::initialize(
        url,
        config(&namespace),
        MessagingKeyring::new(storage_key("initial", 41)),
        clock.clone(),
    )
    .await
    .unwrap();
    initial.close().await;

    let rotated = PostgresRecurringStore::connect(
        url,
        config(&namespace),
        MessagingKeyring::new(storage_key("rotated", 42))
            .with_decryption_key(storage_key("initial", 41))
            .unwrap(),
        clock.clone(),
    )
    .await
    .unwrap();
    rotated.close().await;

    let retired = PostgresRecurringStore::connect(
        url,
        config(&namespace),
        MessagingKeyring::new(storage_key("rotated", 42)),
        clock.clone(),
    )
    .await
    .expect("the initial key is no longer needed for the configuration binding");
    assert!(retired.schedules(None, 1).await.unwrap().is_empty());
    retired.close().await;

    // A keyring that cannot open the binding still fails closed.
    assert!(
        PostgresRecurringStore::connect(
            url,
            config(&namespace),
            MessagingKeyring::new(storage_key("initial", 41)),
            clock,
        )
        .await
        .is_err()
    );
}
