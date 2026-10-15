//! Process-wide state: this binary holds a single test, so no other test can
//! install a Rustls provider first.
#![cfg(feature = "redis-streams")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use rullst_messaging::{BrokerConfig, RedisBroker, RedisBrokerConfig};
use std::time::Duration;

fn configuration(endpoint: &str) -> RedisBrokerConfig {
    RedisBrokerConfig::try_new(
        BrokerConfig::try_new("provider").unwrap(),
        "generation-1",
        endpoint,
        "default",
        "fixture-password",
    )
    .unwrap()
    .with_timeout(Duration::from_secs(2))
    .unwrap()
}

/// Port 1 on loopback refuses the connection; the provider choice happens
/// before connecting.
#[tokio::test]
async fn only_tls_connections_install_a_default_crypto_provider() {
    let plaintext = configuration("redis://127.0.0.1:1")
        .allow_loopback_for_tests()
        .unwrap();
    assert!(RedisBroker::connect(plaintext).await.is_err());
    assert!(rustls::crypto::CryptoProvider::get_default().is_none());

    assert!(
        RedisBroker::connect(configuration("rediss://127.0.0.1:1"))
            .await
            .is_err()
    );
    assert!(rustls::crypto::CryptoProvider::get_default().is_some());
}
