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

/// A throwaway self-signed P-256 CA certificate, valid until 2126.
const TEST_CA: &str = concat!(
    "-----BEGIN CERTIFICATE-----\n",
    "MIIBiDCCAS+gAwIBAgIUflSK31tnN2guJ7teA0JuEa1H0FQwCgYIKoZIzj0EAwIw\n",
    "GTEXMBUGA1UEAwwOUnVsbHN0IHRlc3QgQ0EwIBcNMjYxMDEyMDY0MTE2WhgPMjEy\n",
    "NjA5MTgwNjQxMTZaMBkxFzAVBgNVBAMMDlJ1bGxzdCB0ZXN0IENBMFkwEwYHKoZI\n",
    "zj0CAQYIKoZIzj0DAQcDQgAEq+GaEFF8OtHecQLzmINL6sj8bBZgKKPWtU1MlMxF\n",
    "EANt1B3aS/SZfYe/B4DoXt45Nvlx0HAcGu7CPkBQaYGexaNTMFEwHQYDVR0OBBYE\n",
    "FAjmJHbKIwK4tUDRkHCNUOGJPun9MB8GA1UdIwQYMBaAFAjmJHbKIwK4tUDRkHCN\n",
    "UOGJPun9MA8GA1UdEwEB/wQFMAMBAf8wCgYIKoZIzj0EAwIDRwAwRAIgIIhdOSMR\n",
    "pAoHNm99r+WTL5XXES7ylZtgi3VCcvNqdB0CIGsJyRWDnMOwkJ2yQsVAf3OpHzZB\n",
    "B928Zxu3tshcUiHc\n",
    "-----END CERTIFICATE-----\n",
);

fn offline_config() -> TelemetryConfig {
    TelemetryConfig::try_new("service", "mock_test", ["operation"]).unwrap()
}

#[test]
fn configuration_limits_are_inclusive() {
    assert!(TelemetryConfig::try_new("s".repeat(64), "mock_test", ["operation"]).is_ok());
    for service in [String::new(), "s".repeat(65)] {
        assert!(TelemetryConfig::try_new(service, "mock_test", ["operation"]).is_err());
    }
    // `https://`, the host and `/v1/traces`.
    let endpoint = |len: usize| format!("https://{}/v1/traces", "a".repeat(len - 18));
    assert!(TelemetryConfig::try_new("service", endpoint(2_048), ["operation"]).is_ok());
    assert!(TelemetryConfig::try_new("service", endpoint(2_049), ["operation"]).is_err());
    for endpoint in [
        "https://user@collector.example/v1/traces",
        "https://:secret@collector.example/v1/traces",
        "https://collector.example:0/v1/traces",
    ] {
        assert!(
            TelemetryConfig::try_new("service", endpoint, ["operation"]).is_err(),
            "{endpoint}"
        );
    }
    assert!(
        offline_config()
            .with_limits(32, 32, Duration::from_secs(1), Duration::from_secs(3))
            .is_ok()
    );
    assert!(
        offline_config()
            .with_bearer_token("t".repeat(8_192))
            .is_ok()
    );
    for token in [String::new(), "t".repeat(8_193)] {
        assert!(offline_config().with_bearer_token(token).is_err());
    }
    let debug = format!("{:?}", offline_config());
    assert!(debug.contains("TelemetryConfig"), "{debug}");
    assert!(debug.contains("offline: true"), "{debug}");
}

#[test]
fn ca_bundles_may_fill_64_kib() {
    let bundle = |len: usize| {
        let mut pem = TEST_CA.as_bytes().to_vec();
        pem.resize(len, b'\n');
        pem
    };
    assert!(
        offline_config()
            .with_ca_certificate(TEST_CA.as_bytes().to_vec())
            .is_ok()
    );
    assert!(offline_config().with_ca_certificate(bundle(65_536)).is_ok());
    assert!(
        offline_config()
            .with_ca_certificate(bundle(65_537))
            .is_err()
    );
    assert!(offline_config().with_ca_certificate(Vec::new()).is_err());
}

#[tokio::test]
async fn a_trusted_unsampled_parent_keeps_its_children_unsampled() {
    let runtime = DistributedTelemetry::try_new(
        TelemetryConfig::try_new("fixture", "mock_offline", ["operation"]).unwrap(),
    )
    .unwrap();
    let unsampled = format!("{}-00", &PARENT[..52]);
    let parent = TraceParent::try_new(unsampled.as_str()).unwrap();
    assert_eq!(format!("{parent:?}"), "TraceParent([REDACTED])");
    let subscriber = tracing_subscriber::registry().with(runtime.layer());
    tracing::subscriber::with_default(subscriber, || {
        let span = tracing::info_span!(parent:None,"operation");
        parent.attach_to(&span).unwrap();
        assert!(
            TraceParent::capture(&span)
                .unwrap()
                .as_str()
                .ends_with("-00")
        );
    });
    runtime.flush().await.unwrap();
    let _ = runtime.shutdown().await.unwrap();

    // The separator after the trace id is required on its own.
    let mut separator = PARENT.to_string();
    separator.replace_range(35..36, "a");
    assert!(TraceParent::try_new(separator).is_err());
}
