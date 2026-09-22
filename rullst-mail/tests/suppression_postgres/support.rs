pub use rullst_mail::{
    MailDriver, MailError, MemoryDriver, Message, MutableSuppressionStore,
    PostgresSuppressionConfig, PostgresSuppressionStore, SuppressionError, SuppressionEvent,
    SuppressionGuard, SuppressionKey, SuppressionReason, SuppressionStore,
};
pub fn key() -> SuppressionKey {
    SuppressionKey::new(std::array::from_fn(|i| i as u8)).unwrap()
}
pub fn unique() -> String {
    format!("mail_{:016x}", rand::random::<u64>())
}
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
pub fn config(namespace: &str) -> PostgresSuppressionConfig {
    PostgresSuppressionConfig::new(namespace, 100, 100).unwrap()
}
pub fn event(id: &str, recipient: &str, reason: SuppressionReason, time: u64) -> SuppressionEvent {
    SuppressionEvent::try_new("fixture", id, recipient, reason, time).unwrap()
}
pub fn message(recipient: &str) -> Message {
    Message::new()
        .to(recipient)
        .subject("Account notification")
        .text("Welcome back.")
}
pub fn url() -> String {
    let value = std::env::var("RULLST_MAIL_TEST_POSTGRES_URL").unwrap();
    assert_eq!(
        url::Url::parse(&value).unwrap().path(),
        "/rullst_mail_contract"
    );
    value
}
