use rullst_ai::{
    AiCancellation, AuditDeliveryClient, AuditDeliveryError, AuditDeliveryMode, AuditRetryPolicy,
};
use serde::ser::Error as _;
use std::{net::TcpListener, thread, time::Duration};

mod audit_delivery_support;

use audit_delivery_support::{
    EVENT_TIME_MS, FixtureResponse, TEST_KEY, expected_signature, header, serve,
};

struct FailingEvent;

impl serde::Serialize for FailingEvent {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Err(S::Error::custom("fixture serialization failure"))
    }
}

#[test]
fn configuration_is_bounded_and_debug_is_redacted() {
    assert!(
        AuditDeliveryClient::try_cloud("http://audit.example/v1", "app", "key", TEST_KEY).is_err()
    );
    assert!(
        AuditDeliveryClient::try_local("http://localhost:3000/v1", "app", "key", TEST_KEY).is_err()
    );
    assert!(
        AuditDeliveryClient::try_cloud("https://audit.example/v1?key=x", "app", "key", TEST_KEY)
            .is_err()
    );
    assert!(
        AuditDeliveryClient::try_cloud("https://audit.example/v1", "app", "key", "short").is_err()
    );
    assert!(AuditRetryPolicy::try_new(0, Duration::ZERO).is_err());
    assert!(AuditRetryPolicy::try_new(6, Duration::ZERO).is_err());
    assert!(AuditRetryPolicy::try_new(1, Duration::from_secs(6)).is_err());

    let endpoint = "https://secret-audit.example/v1";
    let client = AuditDeliveryClient::try_cloud(endpoint, "academy", "key-2026", TEST_KEY)
        .expect("valid live audit client");
    let debug = format!("{client:?}");
    assert!(!debug.contains(endpoint));
    assert!(!debug.contains(TEST_KEY));
    assert!(debug.contains("[CONFIGURED]"));
    assert!(debug.contains("[REDACTED]"));
}

#[tokio::test]
async fn offline_mode_is_explicit_bounded_and_never_uses_network() {
    let client = AuditDeliveryClient::try_cloud(
        "https://unreachable.invalid/audit",
        "academy",
        "key-2026",
        "mock_audit",
    )
    .expect("explicit offline client");
    let receipt = client
        .publish(
            "event-001",
            EVENT_TIME_MS,
            &serde_json::json!({"kind": "rag.completed", "raw_prompt": null}),
            &AiCancellation::new(),
        )
        .await
        .expect("offline fixture accepts bounded event");
    assert_eq!(receipt.event_id(), "event-001");
    assert_eq!(receipt.attempts(), 1);
    assert_eq!(receipt.mode(), AuditDeliveryMode::OfflineMock);

    let oversized = "x".repeat(17 * 1_024);
    assert_eq!(
        client
            .publish(
                "event-oversized",
                EVENT_TIME_MS,
                &serde_json::json!({"value": oversized}),
                &AiCancellation::new(),
            )
            .await,
        Err(AuditDeliveryError::EventTooLarge)
    );
    assert_eq!(
        client
            .publish(
                "event-encoding",
                EVENT_TIME_MS,
                &FailingEvent,
                &AiCancellation::new(),
            )
            .await,
        Err(AuditDeliveryError::Encoding)
    );
    assert!(matches!(
        client
            .publish("bad/event", EVENT_TIME_MS, &(), &AiCancellation::new())
            .await,
        Err(AuditDeliveryError::InvalidConfiguration(_))
    ));
    assert!(matches!(
        client
            .publish("event-time", 0, &(), &AiCancellation::new())
            .await,
        Err(AuditDeliveryError::InvalidConfiguration(_))
    ));
    let cancelled = AiCancellation::new();
    cancelled.cancel();
    assert_eq!(
        client
            .publish("event-cancelled", EVENT_TIME_MS, &(), &cancelled)
            .await,
        Err(AuditDeliveryError::Cancelled)
    );
}

#[tokio::test]
async fn live_delivery_retries_with_one_identity_and_verifiable_exact_body_signature() {
    let ack = r#"{"schema_version":1,"event_id":"event-002","accepted":true}"#;
    let (endpoint, requests, server) = serve(vec![
        FixtureResponse {
            status: 503,
            body: String::new(),
            declared_length: None,
            content_type: "application/json",
        },
        FixtureResponse {
            status: 202,
            body: ack.to_string(),
            declared_length: None,
            content_type: "application/json; charset=utf-8",
        },
    ]);
    let retry = AuditRetryPolicy::try_new(2, Duration::ZERO).expect("valid retry policy");
    let client = AuditDeliveryClient::try_local(endpoint, "academy", "key-2026", TEST_KEY)
        .expect("valid loopback client")
        .with_retry_policy(retry);
    let receipt = client
        .publish(
            "event-002",
            EVENT_TIME_MS,
            &serde_json::json!({"kind": "tool.completed", "outcome": "allowed"}),
            &AiCancellation::new(),
        )
        .await
        .expect("second attempt is acknowledged");
    assert_eq!(receipt.mode(), AuditDeliveryMode::Live);
    assert_eq!(receipt.attempts(), 2);

    let first = requests
        .recv_timeout(Duration::from_secs(2))
        .expect("first request");
    let second = requests
        .recv_timeout(Duration::from_secs(2))
        .expect("second request");
    server.join().expect("fixture finishes");
    assert_eq!(first.body, second.body);
    assert_eq!(header(&first.headers, "x-rullst-ai-key-id"), "key-2026");
    assert_eq!(
        header(&first.headers, "x-rullst-ai-timestamp"),
        EVENT_TIME_MS.to_string()
    );
    assert_eq!(
        header(&first.headers, "x-rullst-ai-signature"),
        expected_signature(&first.body)
    );
    assert_eq!(
        header(&second.headers, "x-rullst-ai-signature"),
        expected_signature(&second.body)
    );
    let body: serde_json::Value = serde_json::from_slice(&first.body).expect("valid envelope");
    assert_eq!(body["schema_version"], 1);
    assert_eq!(body["source"], "academy");
    assert_eq!(body["event_id"], "event-002");
    assert_eq!(body["occurred_at_ms"], EVENT_TIME_MS);
}

#[tokio::test]
async fn acknowledgement_must_be_bounded_closed_and_bound_to_the_event() {
    let cases = [
        FixtureResponse {
            status: 202,
            body: r#"{"schema_version":1,"event_id":"different","accepted":true}"#.to_string(),
            declared_length: None,
            content_type: "application/json",
        },
        FixtureResponse {
            status: 202,
            body: r#"{"schema_version":1,"event_id":"event-003","accepted":true,"extra":1}"#
                .to_string(),
            declared_length: None,
            content_type: "application/json",
        },
        FixtureResponse {
            status: 202,
            body: "{}".to_string(),
            declared_length: Some(9 * 1_024),
            content_type: "application/json",
        },
        FixtureResponse {
            status: 202,
            body: r#"{"schema_version":1,"event_id":"event-003","accepted":true}"#.to_string(),
            declared_length: None,
            content_type: "text/plain",
        },
    ];
    for (index, response) in cases.into_iter().enumerate() {
        let oversized = response.declared_length.is_some();
        let (endpoint, _, server) = serve(vec![response]);
        let retry = AuditRetryPolicy::try_new(1, Duration::ZERO).expect("valid retry policy");
        let client = AuditDeliveryClient::try_local(endpoint, "academy", "key-2026", TEST_KEY)
            .expect("valid loopback client")
            .with_retry_policy(retry);
        let error = client
            .publish(
                "event-003",
                EVENT_TIME_MS,
                &serde_json::json!({"case": index}),
                &AiCancellation::new(),
            )
            .await
            .expect_err("invalid acknowledgement fails closed");
        if oversized {
            assert_eq!(error, AuditDeliveryError::AckTooLarge);
        } else {
            assert_eq!(error, AuditDeliveryError::InvalidAck);
        }
        assert!(!error.to_string().contains(TEST_KEY));
        server.join().expect("fixture finishes");
    }
}

#[tokio::test]
async fn cancellation_aborts_an_in_flight_delivery() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback fixture binds");
    let address = listener.local_addr().expect("fixture address resolves");
    let server = thread::spawn(move || {
        let (_stream, _) = listener.accept().expect("fixture accepts request");
        thread::sleep(Duration::from_millis(150));
    });
    let client = AuditDeliveryClient::try_local(
        format!("http://{address}/audit"),
        "academy",
        "key-2026",
        TEST_KEY,
    )
    .expect("valid loopback client");
    let cancellation = AiCancellation::new();
    let trigger = cancellation.clone();
    let task = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(25)).await;
        trigger.cancel();
    });
    assert_eq!(
        client
            .publish(
                "event-004",
                EVENT_TIME_MS,
                &serde_json::json!({"kind": "provider.completed"}),
                &cancellation,
            )
            .await,
        Err(AuditDeliveryError::Cancelled)
    );
    task.await.expect("cancellation task finishes");
    server.join().expect("fixture finishes");
}

#[tokio::test]
async fn permanent_and_exhausted_transient_responses_remain_typed() {
    let (endpoint, requests, server) = serve(vec![FixtureResponse {
        status: 400,
        body: String::new(),
        declared_length: None,
        content_type: "application/json",
    }]);
    let client = AuditDeliveryClient::try_local(endpoint, "academy", "key-2026", TEST_KEY)
        .expect("valid loopback client")
        .with_retry_policy(
            AuditRetryPolicy::try_new(3, Duration::ZERO).expect("valid retry policy"),
        );
    assert_eq!(
        client
            .publish("event-400", EVENT_TIME_MS, &(), &AiCancellation::new())
            .await,
        Err(AuditDeliveryError::Rejected { status: 400 })
    );
    requests
        .recv_timeout(Duration::from_secs(2))
        .expect("one request");
    assert!(requests.try_recv().is_err());
    server.join().expect("fixture finishes");

    let unavailable = FixtureResponse {
        status: 503,
        body: String::new(),
        declared_length: None,
        content_type: "application/json",
    };
    let (endpoint, requests, server) = serve(vec![unavailable.clone(), unavailable]);
    let client = AuditDeliveryClient::try_local(endpoint, "academy", "key-2026", TEST_KEY)
        .expect("valid loopback client")
        .with_retry_policy(
            AuditRetryPolicy::try_new(2, Duration::ZERO).expect("valid retry policy"),
        );
    assert_eq!(
        client
            .publish("event-503", EVENT_TIME_MS, &(), &AiCancellation::new())
            .await,
        Err(AuditDeliveryError::Rejected { status: 503 })
    );
    requests
        .recv_timeout(Duration::from_secs(2))
        .expect("first request");
    requests
        .recv_timeout(Duration::from_secs(2))
        .expect("second request");
    server.join().expect("fixture finishes");
}

#[tokio::test]
async fn deadline_and_retry_wait_cancellation_fail_closed() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback fixture binds");
    let address = listener.local_addr().expect("fixture address resolves");
    let server = thread::spawn(move || {
        let (_stream, _) = listener.accept().expect("fixture accepts request");
        thread::sleep(Duration::from_millis(100));
    });
    let client = AuditDeliveryClient::try_local(
        format!("http://{address}/audit"),
        "academy",
        "key-2026",
        TEST_KEY,
    )
    .expect("valid loopback client")
    .try_with_request_timeout(Duration::from_millis(20))
    .expect("valid short deadline")
    .with_retry_policy(AuditRetryPolicy::try_new(1, Duration::ZERO).expect("valid retry policy"));
    assert_eq!(
        client
            .publish("event-timeout", EVENT_TIME_MS, &(), &AiCancellation::new())
            .await,
        Err(AuditDeliveryError::Deadline)
    );
    server.join().expect("fixture finishes");

    let (endpoint, _, server) = serve(vec![FixtureResponse {
        status: 503,
        body: String::new(),
        declared_length: None,
        content_type: "application/json",
    }]);
    let client = AuditDeliveryClient::try_local(endpoint, "academy", "key-2026", TEST_KEY)
        .expect("valid loopback client")
        .with_retry_policy(
            AuditRetryPolicy::try_new(2, Duration::from_secs(1)).expect("valid retry policy"),
        );
    let cancellation = AiCancellation::new();
    let trigger = cancellation.clone();
    let task = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(25)).await;
        trigger.cancel();
    });
    assert_eq!(
        client
            .publish("event-retry-cancel", EVENT_TIME_MS, &(), &cancellation)
            .await,
        Err(AuditDeliveryError::Cancelled)
    );
    task.await.expect("cancellation task finishes");
    server.join().expect("fixture finishes");
}

#[test]
fn request_timeout_configuration_is_bounded() {
    let build = || {
        AuditDeliveryClient::try_cloud(
            "https://audit.example/v1",
            "academy",
            "key-2026",
            "mock_audit",
        )
        .expect("valid offline client")
    };
    assert!(build().try_with_request_timeout(Duration::ZERO).is_err());
    assert!(
        build()
            .try_with_request_timeout(Duration::from_secs(301))
            .is_err()
    );
    assert!(
        build()
            .try_with_request_timeout(Duration::from_secs(300))
            .is_ok()
    );
}
