pub use rullst_messaging::{schedules::*, *};
pub use std::{
    sync::{
        Arc,
        atomic::{AtomicI64, Ordering},
    },
    time::Duration,
};
#[derive(Clone)]
pub struct ManualClock(pub Arc<AtomicI64>);
impl ManualClock {
    pub fn new() -> Self {
        Self(Arc::new(AtomicI64::new(1_800_000_000_000)))
    }
    pub fn advance(&self, delta: i64) {
        self.0.fetch_add(delta, Ordering::SeqCst);
    }
}
impl Clock for ManualClock {
    fn now_millis(&self) -> rullst_messaging::Result<i64> {
        Ok(self.0.load(Ordering::SeqCst))
    }
}
pub fn keys() -> MessagingKeyring {
    MessagingKeyring::new(MessagingStorageKey::try_new("fixture", [37; 32]).unwrap())
}
pub fn unique() -> String {
    format!("recurring_{}", uuid::Uuid::new_v4().simple())
}
pub fn config(namespace: &str) -> RecurringConfig {
    RecurringConfig::new(namespace, 32, 100)
        .unwrap()
        .with_lease(Duration::from_secs(2))
        .unwrap()
}
pub fn definition(name: &str, clock: &ManualClock, policy: MissedRunPolicy) -> RecurringDefinition {
    RecurringDefinition::new(
        name,
        "* * * * *",
        clock.now_millis().unwrap() - 60_000,
        policy,
        ScheduledMessage::new(
            "scheduled",
            "account.reminder",
            b"PRIVATE-SCHEDULE-CONTENT".to_vec(),
        )
        .unwrap()
        .with_header("x-purpose", "fixture")
        .unwrap(),
    )
    .unwrap()
}
pub fn url() -> String {
    let value = std::env::var("RULLST_RECURRING_TEST_POSTGRES_URL").unwrap();
    assert_eq!(
        url::Url::parse(&value).unwrap().path(),
        "/rullst_recurring_contract"
    );
    value
}
pub async fn open(
    url: &str,
    namespace: &str,
    clock: &ManualClock,
) -> PostgresRecurringStore<ManualClock> {
    PostgresRecurringStore::initialize(url, config(namespace), keys(), clock.clone())
        .await
        .unwrap()
}
pub async fn admin(url: &str) -> sqlx::PgPool {
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(url)
        .await
        .unwrap()
}
