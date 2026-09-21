pub use rullst_auth::recovery::api_tokens::*;
pub use rullst_auth::recovery::{
    AuthClock, AuthenticatedRecoveryAccount, RecoveryError, RecoverySecrets, SessionLabel,
};
use std::sync::atomic::{AtomicU64, Ordering};
pub struct Clock(pub AtomicU64);
impl Clock {
    pub fn new() -> Self {
        Self(AtomicU64::new(1_800_000_000))
    }
    pub fn advance(&self, delta: u64) {
        self.0.fetch_add(delta, Ordering::SeqCst);
    }
    pub fn set(&self, time: u64) {
        self.0.store(time, Ordering::SeqCst);
    }
}
impl AuthClock for Clock {
    fn now(&self) -> Result<u64, RecoveryError> {
        Ok(self.0.load(Ordering::SeqCst))
    }
}
pub fn keys() -> RecoverySecrets {
    RecoverySecrets::new([3; 32], [9; 32]).unwrap()
}
pub fn unique() -> String {
    format!("api-{:016x}", rand::random::<u64>())
}
pub fn scopes() -> ApiScopes {
    ApiScopes::new(["orders:read", "orders:write"]).unwrap()
}
pub fn read() -> ApiScopes {
    ApiScopes::new(["orders:read"]).unwrap()
}
pub fn config(namespace: &str) -> ApiTokenConfig {
    ApiTokenConfig::new(namespace, scopes(), 100, 3600).unwrap()
}
pub fn label() -> SessionLabel {
    SessionLabel::new("Reporting integration").unwrap()
}
#[cfg(feature = "api-tokens-sqlite")]
pub fn sqlite_url(path: &std::path::Path) -> String {
    let encoded: String =
        url::form_urlencoded::byte_serialize(path.to_str().unwrap().as_bytes()).collect();
    format!("sqlite:{}", encoded.replace('+', "%20"))
}
pub async fn account(
    service: &ApiTokenService,
    clock: &Clock,
) -> (AuthenticatedRecoveryAccount, String, String) {
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
    (proof, email, password)
}
