use super::{server::Receiver, support::*};

fn storage_key(id: &str, byte: u8) -> MessagingStorageKey {
    MessagingStorageKey::try_new(id, [byte; 32]).unwrap()
}

/// The documented retirement procedure: rotate, drain and purge events under
/// the full keyring, then drop the old key.
#[tokio::test]
async fn storage_rotation_reseals_the_control_record_so_the_first_key_retires() {
    let clock = ManualClock::new();
    let receiver = Receiver::start(clock.clone(), false).await;
    let (path, url) = fixture();
    let namespace = unique();
    let open_with = |keyring: MessagingKeyring| {
        WebhookOutbox::open(
            url.clone(),
            config(&namespace, receiver.destination()),
            key(),
            keyring,
            clock.clone(),
        )
    };

    let original = open_with(MessagingKeyring::new(storage_key("storage-one", 71)))
        .await
        .unwrap();
    let created = clock.now_millis().unwrap();
    original
        .enqueue("before-rotation", "ready", b"{}".to_vec())
        .await
        .unwrap();
    original.close().await;

    let rotated = open_with(
        MessagingKeyring::new(storage_key("storage-two", 72))
            .with_decryption_key(storage_key("storage-one", 71))
            .unwrap(),
    )
    .await
    .unwrap();
    assert!(matches!(
        rotated.dispatch_next("drain").await.unwrap(),
        WebhookDispatch::Accepted { status: 200, .. }
    ));
    clock.advance(86_400_000 + 10);
    assert_eq!(rotated.purge_terminal(created + 1, 10).await.unwrap(), 1);
    rotated.close().await;

    let retired = open_with(MessagingKeyring::new(storage_key("storage-two", 72)))
        .await
        .expect("the first storage key is no longer required");
    retired
        .enqueue("after-rotation", "ready", b"{}".to_vec())
        .await
        .unwrap();
    assert!(matches!(
        retired.dispatch_next("after").await.unwrap(),
        WebhookDispatch::Accepted { status: 200, .. }
    ));
    retired.close().await;
    cleanup(&path);
}
