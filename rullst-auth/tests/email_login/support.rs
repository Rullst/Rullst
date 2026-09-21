pub use rullst_auth::recovery::email_login::*;
pub use rullst_auth::recovery::{RecoveryDeliveryFailure, RecoveryError, RecoverySecrets};
use std::sync::atomic::{AtomicU64, Ordering};

pub struct Clock(pub AtomicU64);
impl Clock {
    pub fn new() -> Self {
        Self(AtomicU64::new(1_800_000_000))
    }
    pub fn advance(&self, seconds: u64) {
        self.0.fetch_add(seconds, Ordering::SeqCst);
    }
    pub fn set(&self, value: u64) {
        self.0.store(value, Ordering::SeqCst);
    }
}
impl EmailLoginClock for Clock {
    fn now(&self) -> Result<u64, RecoveryError> {
        Ok(self.0.load(Ordering::SeqCst))
    }
}
pub fn keys() -> RecoverySecrets {
    RecoverySecrets::new([3; 32], [9; 32]).unwrap()
}
pub fn config(namespace: &str) -> EmailLoginConfig {
    EmailLoginConfig::new(namespace, "https://app.example/login", "/dashboard", 20).unwrap()
}
pub fn unique() -> String {
    format!("login-{:016x}", rand::random::<u64>())
}
#[cfg(feature = "email-login-sqlite")]
pub fn sqlite_url(path: &std::path::Path) -> String {
    let encoded: String =
        url::form_urlencoded::byte_serialize(path.to_str().unwrap().as_bytes()).collect();
    format!("sqlite:{}", encoded.replace('+', "%20"))
}
pub fn token(delivery: &EmailLoginDelivery) -> String {
    let link = delivery.expose_link();
    url::Url::parse(&link)
        .unwrap()
        .query_pairs()
        .find(|(key, _)| key == "token")
        .unwrap()
        .1
        .into_owned()
}
pub async fn account(service: &EmailLoginService, clock: &Clock) -> (String, String, String) {
    let subject = unique();
    let email = format!("{subject}@example.com");
    let password = format!("Fixture_{:032x}", rand::random::<u128>());
    service
        .accounts()
        .register_account(&subject, &email, &password, clock.now().unwrap())
        .await
        .unwrap();
    let proof = service
        .accounts()
        .authenticate(&email, &password)
        .await
        .unwrap()
        .unwrap();
    service
        .set_account_enabled(&proof, true, clock)
        .await
        .unwrap();
    (subject, email, password)
}
pub async fn issue(
    service: &EmailLoginService,
    email: &str,
    browser: &BrowserBinding,
    clock: &Clock,
) -> EmailLoginDelivery {
    service.request_login(email, browser, clock).await.unwrap();
    service.claim_notice(clock).await.unwrap().unwrap()
}
