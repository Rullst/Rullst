#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::*;
use ::http::{HeaderMap, HeaderValue};
use tracing_subscriber::prelude::*;

const PARENT: &str = "00-0123456789abcdef0123456789abcdef-0123456789abcdef-01";

#[test]
fn configuration_is_explicit_bounded_and_redacted() {
    for endpoint in [
        "http://collector.example/v1/traces",
        "https://user:password@collector.example/v1/traces",
        "https://collector.example/",
        "https://collector.example/v1/traces?token=abc",
        "https://collector.example/v1/traces#fragment",
    ] {
        assert!(TelemetryConfig::try_new("service", endpoint, ["operation"]).is_err());
    }
    assert!(TelemetryConfig::try_new("account@example.com", "mock_test", ["operation"]).is_err());
    assert!(TelemetryConfig::try_new("service", "mock_test", ["operation", "operation"]).is_err());
    assert!(TelemetryConfig::try_new("service", "mock_test", ["operation"; 65]).is_err());
    assert!(TelemetryConfig::try_new("service", "mock_test", [] as [&str; 0]).is_err());
    let config = TelemetryConfig::try_new(
        "service",
        "https://private.example/v1/traces",
        ["operation"],
    )
    .unwrap()
    .with_bearer_token("private-fixture-token")
    .unwrap();
    let debug = format!("{config:?}");
    assert!(!debug.contains("private"));
    for ratio in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
        assert!(config.clone().with_sampling(ratio).is_err());
    }
    assert!(
        config
            .clone()
            .with_limits(0, 1, Duration::from_secs(1), Duration::from_secs(1))
            .is_err()
    );
    assert!(
        config
            .clone()
            .with_limits(1, 2, Duration::from_secs(1), Duration::from_secs(1))
            .is_err()
    );
    assert!(
        config
            .clone()
            .with_limits(4097, 1, Duration::from_secs(1), Duration::from_secs(1))
            .is_err()
    );
    assert!(
        config
            .clone()
            .with_limits(128, 129, Duration::from_secs(1), Duration::from_secs(1))
            .is_err()
    );
    assert!(
        config
            .clone()
            .with_limits(1, 1, Duration::ZERO, Duration::from_secs(1))
            .is_err()
    );
    assert!(config.clone().with_bearer_token("token\nsecond").is_err());
    assert!(
        config
            .clone()
            .with_ca_certificate(b"invalid".to_vec())
            .is_err()
    );
    assert!(config.require_production().is_ok());
    assert!(
        TelemetryConfig::try_new("service", "", ["operation"])
            .unwrap()
            .require_production()
            .is_err()
    );
    assert!(
        TelemetryConfig::loopback_for_tests(
            "service",
            "http://localhost:4318/v1/traces",
            ["operation"]
        )
        .is_err()
    );
    assert!(
        TelemetryConfig::loopback_for_tests(
            "service",
            "http://127.0.0.1:4318/v1/traces",
            ["operation"]
        )
        .unwrap()
        .require_production()
        .is_err()
    );
}

#[test]
fn canonical_parent_rejects_duplicates_and_strips_vendor_metadata() {
    let parent = TraceParent::try_new(PARENT).unwrap();
    assert!(!format!("{parent:?}").contains("012345"));
    let mut headers = HeaderMap::new();
    headers.insert("baggage", HeaderValue::from_static("private=yes"));
    headers.insert("tracestate", HeaderValue::from_static("vendor=private"));
    headers.insert(
        "authorization",
        HeaderValue::from_static("application-owned"),
    );
    parent.inject(&mut headers).unwrap();
    assert!(!headers.contains_key("baggage"));
    assert!(!headers.contains_key("tracestate"));
    assert!(headers.contains_key("authorization"));
    assert_eq!(
        TraceParent::from_headers(&headers, ParentPolicy::TrustedPeer).unwrap(),
        Some(parent.clone())
    );
    assert_eq!(
        TraceParent::from_headers(&headers, ParentPolicy::StartNew).unwrap(),
        None
    );
    headers.append("traceparent", HeaderValue::from_static(PARENT));
    assert_eq!(
        TraceParent::from_headers(&headers, ParentPolicy::TrustedPeer),
        Err(TelemetryError::InvalidContext)
    );
    assert_eq!(
        TraceParent::from_headers(&headers, ParentPolicy::StartNew).unwrap(),
        None
    );
    for invalid in [
        PARENT.to_uppercase(),
        PARENT.replace("00-", "01-"),
        format!("{PARENT}-extra"),
        PARENT.replace(
            "0123456789abcdef0123456789abcdef",
            "00000000000000000000000000000000",
        ),
        PARENT.replace("-0123456789abcdef-", "-0000000000000000-"),
        PARENT.replace("abcdef", "xyzxyz"),
    ] {
        assert!(TraceParent::try_new(invalid).is_err());
    }
}

#[tokio::test]
async fn offline_is_explicit_and_parent_trust_is_an_application_decision() {
    let runtime = DistributedTelemetry::try_new(
        TelemetryConfig::try_new("fixture", "mock_offline", ["operation"]).unwrap(),
    )
    .unwrap();
    assert!(runtime.is_mock());
    let subscriber = tracing_subscriber::registry().with(runtime.layer());
    tracing::subscriber::with_default(subscriber, || {
        let mut headers = HeaderMap::new();
        headers.insert("traceparent", HeaderValue::from_static(PARENT));
        let trusted = tracing::info_span!(parent:None,"operation");
        TraceParent::apply_incoming(&trusted, &headers, ParentPolicy::TrustedPeer).unwrap();
        assert_eq!(
            &TraceParent::capture(&trusted).unwrap().as_str()[3..35],
            &PARENT[3..35]
        );
        let root = tracing::info_span!(parent:None,"operation");
        TraceParent::apply_incoming(&root, &headers, ParentPolicy::StartNew).unwrap();
        assert_ne!(
            &TraceParent::capture(&root).unwrap().as_str()[3..35],
            &PARENT[3..35]
        );
        let rejected = tracing::info_span!("unapproved");
        drop(rejected);
    });
    runtime.flush().await.unwrap();
    let status = runtime.shutdown().await.unwrap();
    assert_eq!(status.forwarded_spans, 2);
    assert_eq!(status.offline_spans, 2);
    assert_eq!(status.rejected_spans, 1);
    assert_eq!(status.accepted_batches, 0);
    assert_eq!(status.failed_batches, 0);
}

#[tokio::test]
async fn configured_sampling_cannot_be_forced_by_an_untrusted_header() {
    let runtime = DistributedTelemetry::try_new(
        TelemetryConfig::try_new("fixture", "mock_offline", ["operation"])
            .unwrap()
            .with_sampling(0.0)
            .unwrap(),
    )
    .unwrap();
    let subscriber = tracing_subscriber::registry().with(runtime.layer());
    tracing::subscriber::with_default(subscriber, || {
        let mut headers = HeaderMap::new();
        headers.insert("traceparent", HeaderValue::from_static(PARENT));
        let span = tracing::info_span!(parent:None,"operation");
        TraceParent::apply_incoming(&span, &headers, ParentPolicy::StartNew).unwrap();
        assert!(
            TraceParent::capture(&span)
                .unwrap()
                .as_str()
                .ends_with("-00")
        );
    });
    runtime.flush().await.unwrap();
    assert_eq!(runtime.shutdown().await.unwrap().forwarded_spans, 0);
}
