#![cfg(feature = "webhooks")]
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "webhook/server.rs"]
mod server;
#[path = "webhook/support.rs"]
mod support;
use server::Receiver;
#[path = "webhook/failures.rs"]
mod failures;
use support::*;

#[tokio::test]
async fn receiver_deduplicates_exact_bytes_after_uncertain_http_acceptance() {
    let clock = ManualClock::new();
    let receiver = Receiver::start(clock.clone(), false).await;
    let (path, url) = fixture();
    let namespace = unique();
    let first = open(&url, &namespace, receiver.destination(), &clock).await;
    let second = open(&url, &namespace, receiver.destination(), &clock).await;
    let body = br#"{ "type": "invoice.ready", "value":"PRIVATE-WEBHOOK-CONTENT" }"#;
    let original = first
        .enqueue("domain-event-1", "invoice.ready", body.to_vec())
        .await
        .unwrap();
    assert!(
        second
            .enqueue("domain-event-1", "invoice.ready", body.to_vec())
            .await
            .unwrap()
            .is_duplicate()
    );
    assert_eq!(
        second
            .enqueue("domain-event-1", "invoice.ready", b"{}".to_vec())
            .await,
        Err(WebhookError::Conflict)
    );
    receiver.drop_once.store(true, Ordering::SeqCst);
    assert!(matches!(
        first.dispatch_next("one").await.unwrap(),
        WebhookDispatch::RetryScheduled { .. }
    ));
    assert_eq!(
        second.dispatch_next("two").await.unwrap(),
        WebhookDispatch::Idle
    );
    clock.advance(3000);
    assert!(matches!(
        second.dispatch_next("two").await.unwrap(),
        WebhookDispatch::Accepted {
            offline: false,
            status: 200,
            ..
        }
    ));
    assert_eq!(receiver.effects.lock().await.len(), 1);
    let requests = receiver.requests.lock().await;
    assert_eq!(requests.len(), 2);
    assert!(requests.iter().all(|r| r.body == body));
    assert!(
        requests
            .iter()
            .all(|r| r.headers["rullst-webhook-id"] == original.id().as_str())
    );
    drop(requests);
    first.close().await;
    second.close().await;
    let reopened = open(&url, &namespace, receiver.destination(), &clock).await;
    assert_eq!(
        reopened.dispatch_next("restart").await.unwrap(),
        WebhookDispatch::Idle
    );
    // Configuration/key drift cannot redirect retained messages.
    let other = WebhookDestination::loopback_test(format!("{}/changed", receiver.url)).unwrap();
    assert!(
        WebhookOutbox::open(
            &url,
            config(&namespace, other),
            key(),
            storage(),
            clock.clone()
        )
        .await
        .is_err()
    );
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .unwrap();
    let payloads: Vec<Vec<u8>> =
        sqlx::query_scalar("SELECT payload FROM rullst_messaging_messages")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert!(
        !payloads
            .iter()
            .any(|v| v.windows(7).any(|w| w == b"PRIVATE"))
    );
    let ids:Vec<String>=sqlx::query_scalar("SELECT idempotency_key FROM rullst_messaging_messages WHERE topic='rullst.webhook.events.v1'").fetch_all(&pool).await.unwrap();
    assert!(ids.iter().all(|id| id != "domain-event-1"));
    pool.close().await;
    reopened.close().await;
    cleanup(&path);
}

#[tokio::test]
async fn redirects_are_terminal_cancellation_fences_and_retention_preserves_control() {
    let clock = ManualClock::new();
    let receiver = Receiver::start(clock.clone(), false).await;
    let (path, url) = fixture();
    let namespace = unique();
    let store = open(&url, &namespace, receiver.destination(), &clock).await;
    receiver.status.store(302, Ordering::SeqCst);
    let event = store
        .enqueue("redirect", "ready", b"{}".to_vec())
        .await
        .unwrap();
    assert!(matches!(
        store.dispatch_next("one").await.unwrap(),
        WebhookDispatch::DeadLettered { .. }
    ));
    assert_eq!(receiver.requests.lock().await.len(), 1);
    let failures = store.failed(10).await.unwrap();
    assert_eq!(failures[0].failure_code(), "webhook.receiver_rejected");
    store.retry_failed(event.id().as_str()).await.unwrap();
    store.cancel(event.id().as_str()).await.unwrap();
    assert!(store.retry_failed(event.id().as_str()).await.is_err());
    assert_eq!(
        store.dispatch_next("one").await.unwrap(),
        WebhookDispatch::Idle
    );
    assert!(
        store
            .purge_terminal(clock.now_millis().unwrap(), 10)
            .await
            .is_err()
    );
    clock.advance(86_400_001);
    assert_eq!(
        store
            .purge_terminal(clock.now_millis().unwrap() - 86_400_000, 10)
            .await
            .unwrap(),
        1
    );
    store.close().await;
    assert!(
        WebhookOutbox::open(
            &url,
            config(
                &namespace,
                WebhookDestination::loopback_test(format!("{}/different", receiver.url)).unwrap()
            ),
            key(),
            storage(),
            clock.clone()
        )
        .await
        .is_err()
    );
    cleanup(&path);
}

#[tokio::test]
async fn verified_tls_is_required_and_owned_certificate_trust_is_explicit() {
    let clock = ManualClock::new();
    let receiver = Receiver::start(clock.clone(), true).await;
    let (path, url) = fixture();
    let namespace = unique();
    let trusted = open(&url, &namespace, receiver.destination(), &clock).await;
    trusted
        .enqueue("tls", "ready", b"{}".to_vec())
        .await
        .unwrap();
    assert!(matches!(
        trusted.dispatch_next("one").await.unwrap(),
        WebhookDispatch::Accepted { offline: false, .. }
    ));
    let untrusted = open(
        &url,
        &unique(),
        WebhookDestination::loopback_test(&receiver.url).unwrap(),
        &clock,
    )
    .await;
    untrusted
        .enqueue("tls", "ready", b"{}".to_vec())
        .await
        .unwrap();
    assert!(matches!(
        untrusted.dispatch_next("one").await.unwrap(),
        WebhookDispatch::RetryScheduled { .. }
    ));
    assert_eq!(receiver.requests.lock().await.len(), 1);
    trusted.close().await;
    untrusted.close().await;
    cleanup(&path);
}

#[tokio::test]
async fn receiver_signature_binds_every_field_and_replay_needs_its_own_state() {
    let clock = ManualClock::new();
    let receiver = Receiver::start(clock.clone(), false).await;
    let (path, url) = fixture();
    let store = open(&url, &unique(), receiver.destination(), &clock).await;
    store
        .enqueue("signature", "ready", b"{\"protected\":true}".to_vec())
        .await
        .unwrap();
    store.dispatch_next("one").await.unwrap();
    let request = receiver.requests.lock().await[0].clone();
    let original = || {
        WebhookSignature::from_headers(
            &request.headers["rullst-webhook-id"],
            &request.headers["rullst-webhook-type"],
            &request.headers["rullst-webhook-timestamp"],
            &request.headers["rullst-webhook-key-id"],
            &request.headers["rullst-webhook-signature"],
        )
        .unwrap()
    };
    let mut header_map = reqwest::header::HeaderMap::new();
    for (name, value) in &request.headers {
        header_map.insert(
            reqwest::header::HeaderName::from_bytes(name.as_bytes()).unwrap(),
            value.parse().unwrap(),
        );
    }
    assert!(WebhookSignature::from_http_headers(&header_map).is_ok());
    header_map.append(
        "rullst-webhook-id",
        request.headers["rullst-webhook-id"].parse().unwrap(),
    );
    assert!(WebhookSignature::from_http_headers(&header_map).is_err());
    let signature = original();
    let skew = Duration::from_secs(300);
    assert!(
        key()
            .verify(&signature, &request.body, &clock, skew)
            .is_ok()
    );
    assert!(
        key()
            .verify(&signature, &request.body, &clock, skew)
            .is_ok()
    ); // cryptography alone does not record replay
    assert!(key().verify(&signature, b"{}", &clock, skew).is_err());
    assert!(
        key()
            .verify(&signature, &vec![0; 65537], &clock, skew)
            .is_err()
    );
    for (field, value) in [
        ("rullst-webhook-id", "msg_00000000000000000000000000000000"),
        ("rullst-webhook-type", "other"),
        ("rullst-webhook-timestamp", "1800000001"),
        ("rullst-webhook-key-id", "other"),
        (
            "rullst-webhook-signature",
            "v1=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        ),
    ] {
        let mut headers = request.headers.clone();
        headers.insert(field.into(), value.into());
        let changed = WebhookSignature::from_headers(
            &headers["rullst-webhook-id"],
            &headers["rullst-webhook-type"],
            &headers["rullst-webhook-timestamp"],
            &headers["rullst-webhook-key-id"],
            &headers["rullst-webhook-signature"],
        )
        .unwrap();
        assert!(
            key().verify(&changed, &request.body, &clock, skew).is_err(),
            "{field}"
        );
    }
    for timestamp in ["-1", "+1800000000", "01800000000", "99999999999999"] {
        assert!(
            WebhookSignature::from_headers(
                signature.delivery_id(),
                signature.event_kind(),
                timestamp,
                signature.key_id(),
                signature.signature_header()
            )
            .is_err()
        );
    }
    clock.advance(301_000);
    assert!(
        key()
            .verify(&signature, &request.body, &clock, skew)
            .is_err()
    );
    clock.advance(-602_000);
    assert!(
        key()
            .verify(&signature, &request.body, &clock, skew)
            .is_err()
    );
    assert!(
        key()
            .verify(&signature, &request.body, &clock, Duration::from_secs(301))
            .is_err()
    );
    assert!(
        WebhookSigningKey::new("fixture", "")
            .unwrap()
            .verify(&signature, &request.body, &clock, skew)
            .is_err()
    );
    store.close().await;
    cleanup(&path);
}

#[test]
fn destination_and_credential_inputs_are_bounded_and_unambiguous() {
    for url in [
        "http://receiver.example/events",
        "file:///private",
        "https://user:secret@example.com/events",
        "https://example.com/#fragment",
        "https://example.com:0/events",
        " https://example.com/events",
        "https://example.com/\nevents",
        "https://localhost/events",
    ] {
        assert!(WebhookDestination::approved_https(url).is_err(), "{url}");
    }
    assert!(WebhookDestination::loopback_test("http://localhost/events").is_err());
    assert!(WebhookDestination::loopback_test("http://192.0.2.1/events").is_err());
    assert!(WebhookSigningKey::new("bad/id", "").is_err());
    assert!(WebhookSigningKey::new("one", "not-a-canonical-key").is_err());
    assert!(WebhookSigningKey::new("one", "x".repeat(129)).is_err());
    assert!(format!("{:?}", key()).contains("REDACTED"));
    assert!(
        WebhookConfig::new(
            "one",
            WebhookDestination::approved_https("https://receiver.example/").unwrap(),
            0
        )
        .is_err()
    );
}
