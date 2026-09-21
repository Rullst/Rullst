use super::redis_support::*;
use rullst_messaging::*;

#[tokio::test]
#[ignore = "requires the owned TLS Redis fixture"]
async fn verified_tls_rejects_untrusted_roots_and_wrong_hostnames() {
    let url = std::env::var("RULLST_MESSAGING_TEST_REDIS_TLS_URL").expect("TLS endpoint");
    let ca = std::fs::read(std::env::var("RULLST_MESSAGING_TEST_REDIS_CA").expect("fixture CA"))
        .unwrap();
    let namespace = unique("tls");
    let config = RedisBrokerConfig::try_new(
        BrokerConfig::try_new(&namespace).unwrap(),
        "fixture-generation",
        url.replace("127.0.0.1", "localhost"),
        "default",
        PASSWORD,
    )
    .unwrap();
    assert!(
        RedisBroker::provision(config.clone().require_production().unwrap())
            .await
            .is_err(),
        "untrusted CA rejected"
    );
    assert!(config.clone().with_ca_certificate(Vec::new()).is_err());
    assert!(
        config
            .clone()
            .with_ca_certificate(b"-----BEGIN PRIVATE KEY-----".to_vec())
            .is_err()
    );
    let verified = config
        .with_ca_certificate(ca.clone())
        .unwrap()
        .require_production()
        .unwrap();
    let broker = RedisBroker::provision(verified.clone()).await.unwrap();
    assert!(!broker.is_mock());
    let receipt = broker.publish(publication("secure", "one")).await.unwrap();
    let other = RedisBroker::connect(verified).await.unwrap();
    let replay = other.publish(publication("secure", "one")).await.unwrap();
    assert_eq!(receipt.id(), replay.id());
    assert!(replay.is_duplicate());
    let wrong_host = RedisBrokerConfig::try_new(
        BrokerConfig::try_new(namespace).unwrap(),
        "fixture-generation",
        url,
        "default",
        PASSWORD,
    )
    .unwrap()
    .with_ca_certificate(ca)
    .unwrap()
    .require_production()
    .unwrap();
    assert!(
        RedisBroker::connect(wrong_host).await.is_err(),
        "CA trust never disables hostname validation"
    );
}
