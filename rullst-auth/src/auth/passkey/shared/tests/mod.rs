use super::*;
use crate::auth::passkey::{
    PasskeyAuth, PasskeyConfig, RegisterPublicKeyCredential,
    test_support::{
        RegistrationFixture, RegistrationOptions, assertion_for_challenge, registration_fixture,
    },
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};
mod browser;
#[path = "journey.rs"]
mod journey;
#[path = "storage.rs"]
mod storage;
mod transport;

type Store = PostgresCeremonyStore<TestClock>;
#[derive(Clone)]
struct TestClock(Arc<AtomicI64>);
impl TestClock {
    fn new() -> Self {
        Self(Arc::new(AtomicI64::new(1000)))
    }
    fn set(&self, now: i64) {
        self.0.store(now, Ordering::SeqCst);
    }
}
impl CeremonyClock for TestClock {
    fn now(&self) -> Result<i64, PasskeyCeremonyError> {
        Ok(self.0.load(Ordering::SeqCst))
    }
}
use PasskeyCeremonyError as Error;
fn config() -> CeremonyStoreConfig {
    CeremonyStoreConfig::new("test-epoch", 8, 30).unwrap()
}
fn auth_config() -> PasskeyConfig {
    PasskeyConfig::new("Test", "localhost", "http://localhost")
        .with_max_pending_challenges(8)
        .with_challenge_ttl_seconds(30)
}
fn binding() -> PasskeyBinding {
    PasskeyBinding::new("tenant-a", "subject-a", "session-a", vec![7; 32]).unwrap()
}
fn intent(id: u8) -> CeremonyIntent {
    CeremonyIntent::new([id; 32], [7; 32], CeremonyKind::Registration, vec![]).unwrap()
}
fn registered_fixture(challenge: &str) -> RegistrationFixture {
    let mut fixture = registration_fixture(
        &PasskeyAuth::new(&auth_config()).unwrap(),
        "localhost",
        RegistrationOptions::default(),
    );
    let bytes = URL_SAFE_NO_PAD
        .decode(&fixture.credential.response.client_data_json)
        .unwrap();
    let mut data: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    data["challenge"] = serde_json::Value::String(challenge.to_owned());
    fixture.credential.response.client_data_json =
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&data).unwrap());
    fixture
}
async fn reset(url: &str) -> (sqlx::PgPool, Store, Store, TestClock) {
    let raw = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(url)
        .await
        .unwrap();
    sqlx::query("DROP SCHEMA IF EXISTS rullst_passkey_ceremony CASCADE")
        .execute(&raw)
        .await
        .unwrap();
    assert!(matches!(
        Store::connect_with_clock(url, config(), TestClock::new()).await,
        Err(Error::Configuration)
    ));
    let clock = TestClock::new();
    let a = Store::initialize_with_clock(url, config(), clock.clone())
        .await
        .unwrap();
    let b = Store::connect_with_clock(url, config(), clock.clone())
        .await
        .unwrap();
    (raw, a, b, clock)
}

#[tokio::test]
#[ignore = "owned disposable PostgreSQL runner only"]
async fn postgres_contract() {
    assert_eq!(
        std::env::var("RULLST_PASSKEY_POSTGRES_DISPOSABLE").as_deref(),
        Ok("1")
    );
    let url = std::env::var("RULLST_PASSKEY_TEST_POSTGRES_URL").unwrap();
    assert!(
        url.starts_with("postgres://postgres@127.0.0.1:")
            && url.ends_with("/rullst_passkey_contract")
    );
    if std::env::var("RULLST_PASSKEY_POSTGRES_PHASE").as_deref() == Ok("restart") {
        let store = Store::connect_with_clock(&url, config(), TestClock::new())
            .await
            .unwrap();
        assert!(
            store
                .consume([81; 32], [7; 32], CeremonyKind::Registration)
                .await
                .is_ok()
        );
        assert!(matches!(
            store
                .consume([82; 32], [7; 32], CeremonyKind::Registration)
                .await,
            Err(Error::Rejected)
        ));
        store.close().await;
        return;
    }
    journey::independent_managers_bind_registration_and_assertion(&url).await;
    journey::credential_snapshot_and_parallel_completion(&url).await;
    journey::completion_rechecks_expiry_after_cryptography(&url).await;
    storage::quotas_expiry_configuration_and_corruption(&url).await;
    storage::lock_wait_expiry_cancellation_and_process(&url).await;
    transport::stalled_transport_has_a_whole_operation_deadline(&url).await;
    browser::run(&url).await;
    let (raw, a, b, _) = reset(&url).await;
    a.issue(&intent(81)).await.unwrap();
    a.issue(&intent(82)).await.unwrap();
    b.consume([82; 32], [7; 32], CeremonyKind::Registration)
        .await
        .unwrap();
    a.close().await;
    b.close().await;
    raw.close().await;
}

#[tokio::test]
#[ignore = "invoked only by the parent-owned PostgreSQL fixture"]
async fn process_completion() {
    assert_eq!(
        std::env::var("RULLST_PASSKEY_CHILD").as_deref(),
        Ok("consume")
    );
    let url = std::env::var("RULLST_PASSKEY_TEST_POSTGRES_URL").unwrap();
    let store = Store::connect_with_clock(url, config(), TestClock::new())
        .await
        .unwrap();
    store
        .consume([77; 32], [7; 32], CeremonyKind::Registration)
        .await
        .unwrap();
    store.close().await;
}
