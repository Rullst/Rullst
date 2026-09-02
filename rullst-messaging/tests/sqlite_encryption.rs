#![cfg(feature = "sqlite")]

mod sqlite_support;
mod support;

use rullst_messaging::{
    MessageAdmin, MessageBroker, MessagingError, MessagingKeyring, MessagingStorageKey,
    PublishRequest, PurgeRequest, ReceiveRequest, SqliteBroker, StartPosition, SubscriptionRequest,
};
use sqlite_support::{cleanup, config, fixture};
use std::time::Duration;
use support::{ManualClock, run_core_contract};

fn keyring(key_id: &str, fill: u8) -> MessagingKeyring {
    MessagingKeyring::new(
        MessagingStorageKey::try_new(key_id, [fill; 32]).expect("valid storage key"),
    )
}

fn rotating_keyring() -> MessagingKeyring {
    keyring("key-2026-02", 2)
        .with_decryption_key(MessagingStorageKey::try_new("key-2026-01", [1; 32]).expect("old key"))
        .expect("rotation keyring")
}

fn receive(topic: &str) -> ReceiveRequest {
    ReceiveRequest::try_new(topic, "workers", "worker-a", 1, Duration::from_secs(30))
        .expect("receive request")
}

#[tokio::test]
async fn encrypted_sqlite_passes_the_shared_broker_contract() {
    let (path, url) = fixture("encrypted-shared-contract");
    let clock = ManualClock::new(50_000);
    clock.advance(0);
    let broker = SqliteBroker::connect_encrypted_with_clock(
        url,
        config("encrypted-shared-contract"),
        keyring("key-1", 7),
        clock,
    )
    .await
    .expect("open encrypted broker");
    run_core_contract(&broker).await;
    drop(broker);
    cleanup(&path);
}

#[tokio::test]
async fn encrypted_instances_serialize_replay_and_competing_claims() {
    let (path, url) = fixture("encrypted-concurrency");
    let clock = ManualClock::new(75_000);
    let first = SqliteBroker::connect_encrypted_with_clock(
        url.clone(),
        config("encrypted-concurrency"),
        keyring("key-1", 7),
        clock.clone(),
    )
    .await
    .expect("open first broker");
    let second = SqliteBroker::connect_encrypted_with_clock(
        url,
        config("encrypted-concurrency"),
        keyring("key-1", 7),
        clock,
    )
    .await
    .expect("open second broker");
    first
        .subscribe(
            SubscriptionRequest::try_new("events", "workers", StartPosition::Earliest)
                .expect("subscription"),
        )
        .await
        .expect("subscribe");
    let request = PublishRequest::try_new("events", "event.ready", "event/1", b"secret".to_vec())
        .expect("publication");
    let (left, right) = tokio::join!(first.publish(request.clone()), second.publish(request));
    let left = left.expect("first publish");
    let right = right.expect("second publish");
    assert_eq!(left.id(), right.id());
    assert_ne!(left.is_duplicate(), right.is_duplicate());

    let (left, right) = tokio::join!(
        first.receive(receive("events")),
        second.receive(receive("events"))
    );
    assert_eq!(
        left.expect("first claim").len() + right.expect("second claim").len(),
        1
    );
    drop(first);
    drop(second);
    cleanup(&path);
}

#[tokio::test]
async fn encrypted_profile_hides_content_and_survives_restart() {
    // TM-MESSAGING-01: raw durable state must not expose protected content.
    let (path, url) = fixture("encrypted-restart");
    let clock = ManualClock::new(100_000);
    let broker = SqliteBroker::connect_encrypted_with_clock(
        url.clone(),
        config("encrypted-restart"),
        keyring("key-1", 7),
        clock.clone(),
    )
    .await
    .expect("open encrypted broker");
    broker
        .subscribe(
            SubscriptionRequest::try_new("events", "workers", StartPosition::Earliest)
                .expect("subscription"),
        )
        .await
        .expect("subscribe");
    broker
        .publish(
            PublishRequest::try_new(
                "events",
                "event.secret",
                "event/secret/1",
                b"private-payload-42".to_vec(),
            )
            .expect("publication")
            .with_header("authorization-hint", "private-header-42")
            .expect("header"),
        )
        .await
        .expect("publish");

    let pool = sqlx::SqlitePool::connect(&url)
        .await
        .expect("inspect database");
    let stored: (String, Vec<u8>) = sqlx::query_as(
        "SELECT headers_json, payload FROM rullst_messaging_messages WHERE namespace = ?",
    )
    .bind("encrypted-restart")
    .fetch_one(&pool)
    .await
    .expect("stored record");
    assert_eq!(stored.0, "rullst.messaging.encrypted.v1:key-1");
    assert!(!stored.0.contains("private-header-42"));
    assert!(
        !stored
            .1
            .windows(b"private-payload-42".len())
            .any(|window| window == b"private-payload-42")
    );
    assert!(
        !stored
            .1
            .windows(b"private-header-42".len())
            .any(|window| window == b"private-header-42")
    );
    pool.close().await;
    drop(broker);

    let reopened = SqliteBroker::connect_encrypted_with_clock(
        url,
        config("encrypted-restart"),
        keyring("key-1", 7),
        clock,
    )
    .await
    .expect("reopen encrypted broker");
    let delivery = reopened
        .receive(receive("events"))
        .await
        .expect("receive after restart")
        .pop()
        .expect("delivery");
    assert_eq!(delivery.envelope().payload(), b"private-payload-42");
    assert_eq!(
        delivery.envelope().headers().get("authorization-hint"),
        Some("private-header-42")
    );
    drop(reopened);
    cleanup(&path);
}

#[tokio::test]
async fn wrong_missing_and_tampered_keys_fail_closed() {
    let (path, url) = fixture("encrypted-authentication");
    let broker = SqliteBroker::connect_encrypted(
        url.clone(),
        config("encrypted-authentication"),
        keyring("key-1", 7),
    )
    .await
    .expect("open encrypted broker");
    drop(broker);

    assert!(matches!(
        SqliteBroker::connect_encrypted(
            url.clone(),
            config("encrypted-authentication"),
            keyring("key-1", 8),
        )
        .await,
        Err(MessagingError::StorageAuthenticationFailed)
    ));
    assert!(matches!(
        SqliteBroker::connect_encrypted(
            url.clone(),
            config("encrypted-authentication"),
            keyring("other-key", 7),
        )
        .await,
        Err(MessagingError::StorageKeyUnavailable)
    ));

    let pool = sqlx::SqlitePool::connect(&url)
        .await
        .expect("tamper database");
    let mut probe: (Vec<u8>,) = sqlx::query_as(
        "SELECT key_probe FROM rullst_messaging_storage_profiles WHERE namespace = ?",
    )
    .bind("encrypted-authentication")
    .fetch_one(&pool)
    .await
    .expect("probe");
    let last = probe.0.len() - 1;
    probe.0[last] ^= 1;
    sqlx::query("UPDATE rullst_messaging_storage_profiles SET key_probe = ? WHERE namespace = ?")
        .bind(probe.0)
        .bind("encrypted-authentication")
        .execute(&pool)
        .await
        .expect("tamper probe");
    pool.close().await;
    assert!(matches!(
        SqliteBroker::connect_encrypted(
            url,
            config("encrypted-authentication"),
            keyring("key-1", 7),
        )
        .await,
        Err(MessagingError::StorageAuthenticationFailed)
    ));
    cleanup(&path);
}

#[tokio::test]
async fn ciphertext_row_substitution_is_rejected_by_metadata_binding() {
    // TM-MESSAGING-02: authenticated row metadata prevents ciphertext swapping.
    let (path, url) = fixture("encrypted-row-binding");
    let broker = SqliteBroker::connect_encrypted(
        url.clone(),
        config("encrypted-row-binding"),
        keyring("key-1", 7),
    )
    .await
    .expect("open encrypted broker");
    broker
        .subscribe(
            SubscriptionRequest::try_new("events", "workers", StartPosition::Earliest)
                .expect("subscription"),
        )
        .await
        .expect("subscribe");
    for sequence in 1..=2 {
        broker
            .publish(
                PublishRequest::try_new(
                    "events",
                    "event.ready",
                    format!("event/{sequence}"),
                    format!("payload-{sequence}").into_bytes(),
                )
                .expect("publication"),
            )
            .await
            .expect("publish");
    }
    let pool = sqlx::SqlitePool::connect(&url)
        .await
        .expect("inspect database");
    let rows: Vec<(i64, Vec<u8>)> = sqlx::query_as(
        "SELECT sequence, payload FROM rullst_messaging_messages WHERE namespace = ? ORDER BY sequence",
    )
    .bind("encrypted-row-binding")
    .fetch_all(&pool)
    .await
    .expect("encrypted rows");
    sqlx::query(
        "UPDATE rullst_messaging_messages SET payload = CASE sequence WHEN 1 THEN ? WHEN 2 THEN ? END WHERE namespace = ?",
    )
    .bind(&rows[1].1)
    .bind(&rows[0].1)
    .bind("encrypted-row-binding")
    .execute(&pool)
    .await
    .expect("swap ciphertext rows");
    pool.close().await;
    assert_eq!(
        broker.receive(receive("events")).await,
        Err(MessagingError::StorageAuthenticationFailed)
    );
    drop(broker);
    cleanup(&path);
}

#[tokio::test]
async fn rotation_requires_old_keys_until_their_records_are_purged() {
    // TM-MESSAGING-03: rotation cannot silently orphan records using prior keys.
    let (path, url) = fixture("encrypted-rotation");
    let clock = ManualClock::new(300_000);
    let old = SqliteBroker::connect_encrypted_with_clock(
        url.clone(),
        config("encrypted-rotation"),
        keyring("key-2026-01", 1),
        clock.clone(),
    )
    .await
    .expect("open old key");
    old.subscribe(
        SubscriptionRequest::try_new("events", "workers", StartPosition::Earliest)
            .expect("subscription"),
    )
    .await
    .expect("subscribe");
    old.publish(
        PublishRequest::try_new("events", "event.old", "old/1", b"old".to_vec())
            .expect("old publication"),
    )
    .await
    .expect("publish old");
    drop(old);

    let rotated = SqliteBroker::connect_encrypted_with_clock(
        url.clone(),
        config("encrypted-rotation"),
        rotating_keyring(),
        clock.clone(),
    )
    .await
    .expect("open rotated keyring");
    rotated
        .publish(
            PublishRequest::try_new("events", "event.new", "new/1", b"new".to_vec())
                .expect("new publication"),
        )
        .await
        .expect("publish new");
    assert!(matches!(
        SqliteBroker::connect_encrypted_with_clock(
            url.clone(),
            config("encrypted-rotation"),
            keyring("key-2026-02", 2),
            clock.clone(),
        )
        .await,
        Err(MessagingError::StorageKeyUnavailable)
    ));

    let old_delivery = rotated
        .receive(receive("events"))
        .await
        .expect("receive old")
        .pop()
        .expect("old delivery");
    assert_eq!(old_delivery.envelope().payload(), b"old");
    rotated
        .ack(old_delivery.ack_token())
        .await
        .expect("ack old");
    assert_eq!(
        rotated
            .purge_terminal(PurgeRequest::try_new("events", 1).expect("purge request"))
            .await
            .expect("purge old")
            .removed(),
        1
    );
    drop(rotated);

    let newest = SqliteBroker::connect_encrypted_with_clock(
        url,
        config("encrypted-rotation"),
        keyring("key-2026-02", 2),
        clock,
    )
    .await
    .expect("drop unused old key");
    assert_eq!(
        newest
            .receive(receive("events"))
            .await
            .expect("receive new")
            .pop()
            .expect("new delivery")
            .envelope()
            .payload(),
        b"new"
    );
    drop(newest);
    cleanup(&path);
}

#[tokio::test]
async fn plaintext_and_encrypted_profiles_never_mix_silently() {
    let (plain_path, plain_url) = fixture("profile-plain");
    let plain = SqliteBroker::connect(plain_url.clone(), config("profile-plain"))
        .await
        .expect("open plaintext");
    drop(plain);
    assert!(matches!(
        SqliteBroker::connect_encrypted(plain_url, config("profile-plain"), keyring("key-1", 1),)
            .await,
        Err(MessagingError::ConfigurationConflict)
    ));
    cleanup(&plain_path);

    let (encrypted_path, encrypted_url) = fixture("profile-encrypted");
    let encrypted = SqliteBroker::connect_encrypted(
        encrypted_url.clone(),
        config("profile-encrypted"),
        keyring("key-1", 1),
    )
    .await
    .expect("open encrypted");
    drop(encrypted);
    assert!(matches!(
        SqliteBroker::connect(encrypted_url, config("profile-encrypted")).await,
        Err(MessagingError::ConfigurationConflict)
    ));
    cleanup(&encrypted_path);
}

#[cfg(unix)]
#[tokio::test]
async fn durable_adapter_rejects_existing_symbolic_link_targets() {
    use std::os::unix::fs::symlink;

    let (target_path, _) = fixture("symlink-target");
    let (link_path, link_url) = fixture("symlink-link");
    symlink(&target_path, &link_path).expect("create symbolic link");
    assert!(matches!(
        SqliteBroker::connect(link_url, config("symlink-target")).await,
        Err(MessagingError::Invalid {
            field: "durable SQLite database URL",
            reason: "existing target must be a regular file",
        })
    ));
    let _ = std::fs::remove_file(&link_path);
    cleanup(&target_path);
}
