use super::*;
use crate::{InMemoryBroker, MessageBroker, PublishRequest};

const TRACEPARENT: &str = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";

#[test]
fn trace_context_accepts_canonical_values_and_redacts_debug() {
    let context = TraceContext::try_with_state(TRACEPARENT, "vendor=value,tenant@rullst=opaque")
        .expect("valid context");
    assert_eq!(context.traceparent(), TRACEPARENT);
    assert_eq!(
        context.tracestate(),
        Some("vendor=value,tenant@rullst=opaque")
    );
    let debug = format!("{context:?}");
    assert!(!debug.contains("4bf92f"));
    assert!(!debug.contains("vendor=value"));
}

#[test]
fn malformed_or_ambiguous_trace_context_fails_closed() {
    // TM-MESSAGING-05: correlation metadata cannot bypass its strict allowlist.
    for invalid in [
        "00-00000000000000000000000000000000-00f067aa0ba902b7-01",
        "00-4bf92f3577b34da6a3ce929d0e0e4736-0000000000000000-01",
        "01-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
        "00-4BF92F3577B34DA6A3CE929D0E0E4736-00f067aa0ba902b7-01",
        "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-0g",
    ] {
        assert!(TraceContext::try_new(invalid).is_err());
    }
    for invalid in [
        " vendor=value",
        "Vendor=value",
        "vendor@=value",
        "vendor@system@other=value",
        "1vendor=value",
        "vendor=value,vendor=duplicate",
        "vendor=bad,value",
        "vendor=bad=value",
        "vendor=line\nbreak",
    ] {
        assert!(TraceContext::try_with_state(TRACEPARENT, invalid).is_err());
    }
}

#[tokio::test]
async fn propagation_uses_only_allowlisted_headers_and_survives_delivery() {
    let context = TraceContext::try_with_state(TRACEPARENT, "vendor=value").expect("context");
    let request = PublishRequest::try_new("events", "event.ready", "event/trace", b"body".to_vec())
        .expect("request")
        .with_trace_context(&context)
        .expect("trace headers");
    assert_eq!(request.headers().len(), 2);
    assert!(request.headers().get("baggage").is_none());
    assert!(request.clone().with_trace_context(&context).is_err());

    let broker = InMemoryBroker::new(crate::BrokerConfig::try_new("trace").expect("config"));
    broker
        .subscribe(
            crate::SubscriptionRequest::try_new(
                "events",
                "workers",
                crate::StartPosition::Earliest,
            )
            .expect("subscription"),
        )
        .await
        .expect("subscribe");
    broker.publish(request).await.expect("publish");
    let delivery = broker
        .receive(
            crate::ReceiveRequest::try_new(
                "events",
                "workers",
                "worker",
                1,
                std::time::Duration::from_secs(1),
            )
            .expect("receive"),
        )
        .await
        .expect("delivery")
        .pop()
        .expect("message");
    assert_eq!(
        delivery
            .envelope()
            .trace_context()
            .expect("valid carried context"),
        Some(context)
    );
}

#[test]
fn traceparent_needs_each_of_its_three_separators() {
    for index in [2, 35, 52] {
        let mut bytes = TRACEPARENT.as_bytes().to_vec();
        bytes[index] = b'a';
        assert_eq!(
            TraceContext::try_new(String::from_utf8(bytes).unwrap()).err(),
            Some(invalid("traceparent must be canonical W3C version 00")),
            "{index}"
        );
    }
}

#[test]
fn tracestate_accepts_exactly_512_bytes_and_32_members() {
    let bounds = Some(invalid("tracestate is outside the supported bounds"));
    let state = |value: String| TraceContext::try_with_state(TRACEPARENT, value);
    // 256 + 1 + 255 bytes.
    let largest = format!("a={},b={}", "x".repeat(254), "x".repeat(253));
    assert_eq!(largest.len(), 512);
    assert!(state(largest.clone()).is_ok());
    assert_eq!(state(format!("{largest}x")).err(), bounds);

    let members = |count: usize| {
        (0..count)
            .map(|index| format!("k{index}=v"))
            .collect::<Vec<_>>()
            .join(",")
    };
    assert!(state(members(32)).is_ok());
    assert_eq!(state(members(33)).err(), bounds);
    assert_eq!(state(String::new()).err(), bounds);
    assert_eq!(state(" vendor=value".to_owned()).err(), bounds);
}

#[test]
fn tracestate_members_hold_one_to_256_bytes() {
    let malformed = Some(invalid("tracestate member is malformed"));
    let state = |value: String| TraceContext::try_with_state(TRACEPARENT, value);
    assert!(state(format!("vendor={}", "x".repeat(256))).is_ok());
    assert_eq!(
        state(format!("vendor={}", "x".repeat(257))).err(),
        malformed
    );
    assert_eq!(state("vendor=".to_owned()).err(), malformed);
}

#[test]
fn tracestate_keys_accept_digits_and_the_four_symbols() {
    for value in ["k0_-*/=v", "0tenant_-*/@sys0_-*/=v"] {
        assert!(
            TraceContext::try_with_state(TRACEPARENT, value).is_ok(),
            "{value}"
        );
    }
}

#[test]
fn debug_output_says_only_whether_a_tracestate_is_present() {
    let plain = TraceContext::try_new(TRACEPARENT).unwrap();
    assert_eq!(
        format!("{plain:?}"),
        "TraceContext { traceparent: \"[REDACTED]\", has_tracestate: false }"
    );
    let with_state = TraceContext::try_with_state(TRACEPARENT, "vendor=value").unwrap();
    assert_eq!(
        format!("{with_state:?}"),
        "TraceContext { traceparent: \"[REDACTED]\", has_tracestate: true }"
    );
}
