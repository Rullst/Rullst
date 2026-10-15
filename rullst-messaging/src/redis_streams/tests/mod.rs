//! Configuration bounds without a server, and reply validation against an
//! in-process fake server.
#![allow(clippy::expect_used, clippy::unwrap_used)]

mod fake;
mod replies;

use super::*;
use crate::{BrokerConfig, MessagingError};

fn limits(subscriptions: usize, payload: usize) -> BrokerConfig {
    BrokerConfig::try_new("limits")
        .and_then(|config| config.with_limits(1, subscriptions, 1, payload))
        .unwrap()
}

fn configure(
    broker: BrokerConfig,
    endpoint: &str,
    username: &str,
    password: &str,
) -> Result<RedisBrokerConfig> {
    RedisBrokerConfig::try_new(broker, "generation-1", endpoint, username, password)
}

fn mock() -> RedisBrokerConfig {
    configure(limits(1, 1), "rediss://redis.example", "", "").unwrap()
}

#[test]
fn profile_limits_cap_groups_and_payloads_independently() {
    assert!(configure(limits(128, 1_048_576), "rediss://redis.example", "", "").is_ok());
    for (subscriptions, payload) in [(129, 1), (1, 1_048_577)] {
        assert_eq!(
            configure(
                limits(subscriptions, payload),
                "rediss://redis.example",
                "",
                ""
            )
            .err(),
            Some(MessagingError::Invalid {
                field: "Redis profile limits",
                reason: "maximums are 10000 messages, 128 groups and one MiB per payload",
            }),
            "{subscriptions} {payload}"
        );
    }
}

#[test]
fn ca_bundles_are_bounded_certificates_without_private_keys() {
    let certificate = |length: usize| {
        let mut pem = b"-----BEGIN CERTIFICATE-----\n".to_vec();
        pem.resize(length, b'A');
        pem
    };
    assert!(mock().with_ca_certificate(certificate(65_536)).is_ok());
    let refused = Some(MessagingError::Invalid {
        field: "Redis CA certificate",
        reason: "requires bounded PEM certificate data",
    });
    for pem in [
        certificate(65_537),
        b"not a certificate".to_vec(),
        b"-----BEGIN CERTIFICATE-----\n-----BEGIN PRIVATE KEY-----\n".to_vec(),
    ] {
        assert_eq!(mock().with_ca_certificate(pem).err(), refused);
    }
}

#[test]
fn credentials_are_bounded_at_exactly_128_and_4096_bytes() {
    let endpoint = "rediss://redis.example";
    let password = format!("mock_{}", "p".repeat(4_091));
    assert!(configure(limits(1, 1), endpoint, &"u".repeat(128), &password).is_ok());
    let refused = Some(MessagingError::Invalid {
        field: "Redis credentials",
        reason: "requires bounded explicit ACL credentials",
    });
    assert_eq!(
        configure(limits(1, 1), endpoint, &"u".repeat(129), "mock_x").err(),
        refused
    );
    assert_eq!(
        configure(limits(1, 1), endpoint, "user", &format!("{password}p")).err(),
        refused
    );
}

#[test]
fn endpoints_are_bounded_at_exactly_2048_bytes_and_carry_no_user() {
    // 31 labels of 63 bytes and one of 55 make a 2,039-byte host.
    let host = format!(
        "{}{}",
        format!("{}.", "a".repeat(63)).repeat(31),
        "a".repeat(55)
    );
    let endpoint = format!("rediss://{host}");
    assert_eq!(endpoint.len(), 2_048);
    assert!(configure(limits(1, 1), &endpoint, "", "").is_ok());
    assert_eq!(
        configure(limits(1, 1), &format!("{endpoint}a"), "", "").err(),
        Some(MessagingError::Invalid {
            field: "Redis endpoint",
            reason: "endpoint is oversized",
        })
    );
    assert_eq!(
        configure(limits(1, 1), "rediss://user@redis.example", "", "").err(),
        Some(MessagingError::Invalid {
            field: "Redis endpoint",
            reason: "requires a credential-free database-zero URL",
        })
    );
}

#[tokio::test]
async fn debug_output_shows_only_the_profile_and_mode() {
    let config = mock();
    let debug = format!("{config:?}");
    assert!(
        debug.starts_with("RedisBrokerConfig { broker: BrokerConfig {"),
        "{debug}"
    );
    assert!(
        debug.ends_with("is_mock: true, timeout: 10s, test_transport: false, .. }"),
        "{debug}"
    );
    let broker = RedisBroker::connect(config).await.unwrap();
    assert_eq!(format!("{broker:?}"), "RedisBroker { is_mock: true, .. }");
}
