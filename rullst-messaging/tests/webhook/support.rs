use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
pub use rullst_messaging::{Clock, MessagingKeyring, MessagingStorageKey, webhooks::*};
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
    pub fn advance(&self, ms: i64) {
        self.0.fetch_add(ms, Ordering::SeqCst);
    }
}
impl Clock for ManualClock {
    fn now_millis(&self) -> rullst_messaging::Result<i64> {
        Ok(self.0.load(Ordering::SeqCst))
    }
}
pub fn key() -> WebhookSigningKey {
    WebhookSigningKey::new("fixture", URL_SAFE_NO_PAD.encode([37; 32])).unwrap()
}
pub fn storage() -> MessagingKeyring {
    MessagingKeyring::new(MessagingStorageKey::try_new("storage", [71; 32]).unwrap())
}
pub fn unique() -> String {
    format!("webhook_{}", uuid::Uuid::new_v4().simple())
}
pub fn fixture() -> (std::path::PathBuf, String) {
    let path = std::env::temp_dir().join(format!("{}.sqlite", unique()));
    let url = format!("sqlite://{}", path.to_string_lossy().replace('\\', "/"));
    (path, url)
}
pub fn cleanup(path: &std::path::Path) {
    for path in [
        path.to_path_buf(),
        path.with_extension("sqlite-wal"),
        path.with_extension("sqlite-shm"),
    ] {
        let _ = std::fs::remove_file(path);
    }
}
pub fn config(namespace: &str, destination: WebhookDestination) -> WebhookConfig {
    WebhookConfig::new(namespace, destination, 16).unwrap()
}
pub async fn open(
    url: &str,
    namespace: &str,
    destination: WebhookDestination,
    clock: &ManualClock,
) -> WebhookOutbox<ManualClock> {
    WebhookOutbox::open(
        url,
        config(namespace, destination),
        key(),
        storage(),
        clock.clone(),
    )
    .await
    .unwrap()
}
