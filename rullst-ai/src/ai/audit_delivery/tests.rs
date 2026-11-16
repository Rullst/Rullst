use super::*;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

const KEY: &str = "0123456789abcdef0123456789abcdef";
const EVENT_TIME_MS: u64 = 1_788_000_000_000;

#[test]
fn retry_policy_bounds_and_backoff_are_exact() {
    assert!(AuditRetryPolicy::try_new(0, Duration::ZERO).is_err());
    assert!(AuditRetryPolicy::try_new(6, Duration::ZERO).is_err());
    assert!(AuditRetryPolicy::try_new(1, Duration::from_secs(6)).is_err());
    assert!(AuditRetryPolicy::try_new(1, Duration::from_secs(5)).is_ok());

    let policy = AuditRetryPolicy::try_new(5, Duration::from_secs(1)).expect("valid policy");
    assert_eq!(policy.delay_for(1), Duration::from_secs(1));
    assert_eq!(policy.delay_for(2), Duration::from_secs(2));
    assert_eq!(policy.delay_for(3), Duration::from_secs(4));
    assert_eq!(policy.delay_for(4), Duration::from_secs(5));
    assert_eq!(policy.delay_for(u8::MAX), Duration::from_secs(5));

    let default = AuditRetryPolicy::default();
    assert_eq!(default.max_attempts, 3);
    assert_eq!(default.base_delay, Duration::from_millis(200));
}

#[test]
fn retry_classification_is_narrow() {
    assert!(retryable_delivery(&AuditDeliveryError::Transport));
    assert!(retryable_delivery(&AuditDeliveryError::Deadline));
    assert!(retryable_delivery(&AuditDeliveryError::Rejected {
        status: 429
    }));
    assert!(retryable_delivery(&AuditDeliveryError::Rejected {
        status: 500
    }));
    assert!(!retryable_delivery(&AuditDeliveryError::Rejected {
        status: 400
    }));
    assert!(!retryable_delivery(&AuditDeliveryError::InvalidAck));
    assert!(!retryable_delivery(&AuditDeliveryError::Cancelled));
}

#[test]
fn hexadecimal_encoding_is_lowercase_and_empty_safe() {
    assert_eq!(hex(&[]), "");
    assert_eq!(hex(&[0x00, 0x0f, 0x10, 0xab, 0xff]), "000f10abff");
}

fn offline_client() -> AuditDeliveryClient {
    AuditDeliveryClient::try_local(
        "http://127.0.0.1:9/audit",
        "academy",
        "key-2026",
        "mock_audit",
    )
    .expect("offline client")
}

#[tokio::test]
async fn event_envelope_and_time_accept_their_exact_ceilings() {
    let client = offline_client();
    let overhead = serde_json::to_vec(&AuditEnvelope {
        schema_version: 1,
        source: "academy",
        event_id: "event-size",
        occurred_at_ms: EVENT_TIME_MS,
        event: &"",
    })
    .expect("envelope")
    .len();
    // A 16,384-byte envelope is accepted, one more byte is refused.
    let fits = "x".repeat(16_384 - overhead);
    let cancellation = AiCancellation::new();
    assert!(
        client
            .publish("event-size", EVENT_TIME_MS, &fits, &cancellation)
            .await
            .is_ok()
    );
    let over = format!("{fits}x");
    assert_eq!(
        client
            .publish("event-size", EVENT_TIME_MS, &over, &cancellation)
            .await,
        Err(AuditDeliveryError::EventTooLarge)
    );

    // The largest signed 64-bit millisecond is the last accepted time.
    assert!(
        client
            .publish("event-time", 9_223_372_036_854_775_807, &(), &cancellation)
            .await
            .is_ok()
    );
    assert!(matches!(
        client
            .publish("event-time", 9_223_372_036_854_775_808, &(), &cancellation)
            .await,
        Err(AuditDeliveryError::InvalidConfiguration(_))
    ));
}

/// Reads one request and answers with `response`; every wait is bounded.
async fn serve(response: String) -> (String, tokio::task::JoinHandle<bool>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("fixture binds");
    let endpoint = format!("http://{}/audit", listener.local_addr().expect("address"));
    let task = tokio::spawn(async move {
        let Ok(Ok((mut stream, _))) =
            tokio::time::timeout(Duration::from_secs(2), listener.accept()).await
        else {
            return false;
        };
        let mut request = Vec::new();
        let mut chunk = [0_u8; 4_096];
        loop {
            let Ok(Ok(count)) =
                tokio::time::timeout(Duration::from_secs(2), stream.read(&mut chunk)).await
            else {
                return false;
            };
            if count == 0 {
                return false;
            }
            request.extend_from_slice(&chunk[..count]);
            let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") else {
                continue;
            };
            let headers = String::from_utf8_lossy(&request[..end]).to_ascii_lowercase();
            let length = headers
                .lines()
                .find_map(|line| line.strip_prefix("content-length: "))
                .and_then(|value| value.trim().parse::<usize>().ok())
                .unwrap_or(0);
            if request.len() >= end + 4 + length {
                break;
            }
        }
        tokio::time::timeout(
            Duration::from_secs(2),
            stream.write_all(response.as_bytes()),
        )
        .await
        .is_ok_and(|written| written.is_ok())
    });
    (endpoint, task)
}

fn ack_response(body: &str, declared: bool) -> String {
    let length = if declared {
        format!("Content-Length: {}\r\n", body.len())
    } else {
        String::new()
    };
    format!(
        "HTTP/1.1 202 Accepted\r\nContent-Type: application/json\r\n{length}Connection: close\r\n\r\n{body}"
    )
}

/// A valid acknowledgement padded with JSON whitespace to `total` bytes.
fn padded_ack(total: usize) -> String {
    let ack = r#"{"schema_version":1,"event_id":"event-ack","accepted":true}"#;
    format!("{ack}{}", " ".repeat(total - ack.len()))
}

async fn deliver(response: String) -> Result<AuditDeliveryReceipt, AuditDeliveryError> {
    let (endpoint, server) = serve(response).await;
    let client = AuditDeliveryClient::try_local(endpoint, "academy", "key-2026", KEY)
        .expect("live loopback client")
        .with_retry_policy(AuditRetryPolicy::try_new(1, Duration::ZERO).expect("one attempt"));
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        client.publish("event-ack", EVENT_TIME_MS, &(), &AiCancellation::new()),
    )
    .await
    .expect("delivery is bounded");
    assert!(server.await.expect("fixture task"), "fixture answered");
    result
}

#[tokio::test]
async fn acknowledgement_limit_is_exactly_8_kib_declared_or_streamed() {
    for declared in [true, false] {
        let receipt = deliver(ack_response(&padded_ack(8_192), declared))
            .await
            .expect("an 8,192-byte acknowledgement is accepted");
        assert_eq!(receipt.attempts(), 1);
        assert_eq!(
            deliver(ack_response(&padded_ack(8_193), declared)).await,
            Err(AuditDeliveryError::AckTooLarge)
        );
    }
}

#[tokio::test]
async fn acknowledgement_needs_version_1_and_acceptance() {
    for ack in [
        r#"{"schema_version":2,"event_id":"event-ack","accepted":true}"#,
        r#"{"schema_version":1,"event_id":"event-ack","accepted":false}"#,
    ] {
        assert_eq!(
            deliver(ack_response(ack, true)).await,
            Err(AuditDeliveryError::InvalidAck)
        );
    }
}
