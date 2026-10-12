//! Email-login boundaries over a disposable SQLite file, including stored rows
//! edited outside the service to prove each consistency check on its own.
use super::super::{RecoveryDeliveryFailure, RecoveryLocale};
use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

#[path = "tests/delivery.rs"]
mod delivery;

const T: u64 = 1_800_000_000;

struct Clock(AtomicU64);
impl Clock {
    fn at(time: u64) -> Self {
        Self(AtomicU64::new(time))
    }
    fn set(&self, time: u64) {
        self.0.store(time, Ordering::SeqCst);
    }
}
impl EmailLoginClock for Clock {
    fn now(&self) -> Result<u64, RecoveryError> {
        Ok(self.0.load(Ordering::SeqCst))
    }
}

fn config() -> EmailLoginConfig {
    EmailLoginConfig::new("login-unit", "https://app.example/login", "/dashboard", 20).unwrap()
}

/// A fresh database file; keep the directory alive for the test.
async fn service() -> (tempfile::TempDir, String, EmailLoginService) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("login.db");
    let encoded: String =
        url::form_urlencoded::byte_serialize(path.to_str().unwrap().as_bytes()).collect();
    let url = format!("sqlite:{}", encoded.replace('+', "%20"));
    let keys = RecoverySecrets::new([3; 32], [9; 32]).unwrap();
    let service = EmailLoginService::initialize(&url, keys, config())
        .await
        .unwrap();
    (directory, url, service)
}

/// Registers and opts in one account, returning its email and proof.
async fn enabled(
    service: &EmailLoginService,
    clock: &Clock,
    name: &str,
    locale: Option<RecoveryLocale>,
) -> (String, AuthenticatedRecoveryAccount) {
    let email = format!("{name}@example.com");
    let password = "Fixture_0123456789abcdef0123456789";
    service
        .accounts()
        .register_account_with_locale(name, &email, password, locale, clock.now().unwrap())
        .await
        .unwrap();
    let proof = service
        .accounts()
        .authenticate(&email, password)
        .await
        .unwrap()
        .unwrap();
    service
        .set_account_enabled(&proof, true, clock)
        .await
        .unwrap();
    (email, proof)
}

async fn execute(service: &EmailLoginService, statement: &'static str) {
    sqlx::query(statement)
        .execute(&service.store.pool)
        .await
        .unwrap();
}

async fn count(service: &EmailLoginService, table: &'static str) -> i64 {
    sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT COUNT(*) FROM {table}")))
        .fetch_one(&service.store.pool)
        .await
        .unwrap()
}

fn link_token(delivery: &EmailLoginDelivery) -> String {
    url::Url::parse(&delivery.expose_link())
        .unwrap()
        .query_pairs()
        .find(|(key, _)| key == "token")
        .unwrap()
        .1
        .into_owned()
}

#[test]
fn landing_urls_are_bounded_and_carry_no_credentials_or_fragment() {
    // Thirty-one 63-byte labels and one of 50 make a 2034-byte host.
    let mut host = format!("{}.", "a".repeat(63)).repeat(31);
    host.push_str(&"b".repeat(50));
    let longest = format!("https://{host}/login");
    assert_eq!(longest.len(), 2_048);
    assert!(EmailLoginConfig::new("login-unit", longest.as_str(), "/dashboard", 20).is_ok());
    let longer = format!("https://{host}/logins");
    for landing in [
        longer.as_str(),
        "https://user@app.example/login",
        "https://:secret@app.example/login",
        "https://app.example/login#fragment",
    ] {
        assert_eq!(
            EmailLoginConfig::new("login-unit", landing, "/dashboard", 20).err(),
            Some(RecoveryError::Configuration),
            "{landing}"
        );
    }
    assert_eq!(
        format!("{:?}", config()),
        "EmailLoginConfig { capacity: 20, development: false, .. }"
    );
}

#[tokio::test]
async fn requests_take_at_least_the_minimum_response_time() {
    let (_directory, _, service) = service().await;
    let clock = Clock::at(T);
    let browser = BrowserBinding::generate().unwrap();
    let started = std::time::Instant::now();
    service
        .request_login("absent@example.com", &browser, &clock)
        .await
        .unwrap();
    assert!(started.elapsed() >= std::time::Duration::from_millis(250));
}

#[tokio::test]
async fn stored_request_counters_and_revisions_are_checked_on_their_own() {
    let (_directory, _, service) = service().await;
    let clock = Clock::at(T);
    let (email, proof) = enabled(&service, &clock, "owner-1", None).await;
    let browser = BrowserBinding::generate().unwrap();
    for (tamper, restore) in [
        (
            "UPDATE rullst_email_login_accounts SET request_window = -1",
            "UPDATE rullst_email_login_accounts SET request_window = 0",
        ),
        (
            "UPDATE rullst_email_login_accounts SET requests = -1",
            "UPDATE rullst_email_login_accounts SET requests = 0",
        ),
        (
            "UPDATE rullst_email_login_accounts SET revision = 0",
            "UPDATE rullst_email_login_accounts SET revision = 1",
        ),
    ] {
        execute(&service, tamper).await;
        assert_eq!(
            service.request_login(&email, &browser, &clock).await.err(),
            Some(RecoveryError::Configuration),
            "{tamper}"
        );
        execute(&service, restore).await;
    }
    // Later policy revisions stay valid for opt-in changes and requests.
    service
        .set_account_enabled(&proof, false, &clock)
        .await
        .unwrap();
    service
        .set_account_enabled(&proof, true, &clock)
        .await
        .unwrap();
    service
        .request_login(&email, &browser, &clock)
        .await
        .unwrap();
    assert!(service.claim_notice(&clock).await.unwrap().is_some());
}

#[tokio::test]
async fn the_namespace_request_budget_counts_each_request_up_to_120() {
    let (_directory, _, service) = service().await;
    let clock = Clock::at(T);
    let (email, _) = enabled(&service, &clock, "owner-1", None).await;
    let browser = BrowserBinding::generate().unwrap();
    execute(
        &service,
        "UPDATE rullst_email_login_control SET request_window = 1800000000, requests = 119",
    )
    .await;
    service
        .request_login("absent@example.com", &browser, &clock)
        .await
        .unwrap();
    let requests: i64 = sqlx::query_scalar("SELECT requests FROM rullst_email_login_control")
        .fetch_one(&service.store.pool)
        .await
        .unwrap();
    assert_eq!(requests, 120);
    service
        .request_login(&email, &browser, &clock)
        .await
        .unwrap();
    assert!(
        service.claim_notice(&clock).await.unwrap().is_none(),
        "the 121st request in the window queues nothing"
    );
    for window in [-1_i64, 1_800_000_100] {
        sqlx::query("UPDATE rullst_email_login_control SET request_window = $1")
            .bind(window)
            .execute(&service.store.pool)
            .await
            .unwrap();
        assert_eq!(
            service.request_login(&email, &browser, &clock).await.err(),
            Some(RecoveryError::Configuration),
            "window {window}"
        );
    }
    execute(
        &service,
        "UPDATE rullst_email_login_control SET request_window = 1800000000",
    )
    .await;
    clock.set(T + 60);
    service
        .request_login(&email, &browser, &clock)
        .await
        .unwrap();
    assert!(service.claim_notice(&clock).await.unwrap().is_some());
}

#[tokio::test]
async fn redemption_rechecks_the_stored_link_and_starts_at_time_zero() {
    let (_directory, _, service) = service().await;
    // A link issued at the epoch itself is valid.
    let clock = Clock::at(0);
    let (email, _) = enabled(&service, &clock, "owner-1", None).await;
    let browser = BrowserBinding::generate().unwrap();
    service
        .request_login(&email, &browser, &clock)
        .await
        .unwrap();
    let delivery = service.claim_notice(&clock).await.unwrap().unwrap();
    clock.set(10);
    let session = service
        .redeem(&link_token(&delivery), &browser, &clock)
        .await
        .unwrap();
    assert_eq!(session.subject(), "owner-1");
    assert_eq!(session.destination(), "/dashboard");
    assert_eq!(format!("{session:?}"), "EmailLoginSession([REDACTED])");

    // A stored lifetime other than 900 seconds is inconsistent.
    clock.set(T);
    service
        .request_login(&email, &browser, &clock)
        .await
        .unwrap();
    let delivery = service.claim_notice(&clock).await.unwrap().unwrap();
    execute(
        &service,
        "UPDATE rullst_email_login_tokens SET expires_at = issued_at + 899",
    )
    .await;
    assert_eq!(
        service
            .redeem(&link_token(&delivery), &browser, &clock)
            .await
            .err(),
        Some(RecoveryError::Configuration)
    );
    // A malformed presented token never matches a row, even one forged for it.
    execute(
        &service,
        "UPDATE rullst_email_login_tokens SET expires_at = issued_at + 900",
    )
    .await;
    let forged = service.digest("email-login-token-v1", "");
    sqlx::query("UPDATE rullst_email_login_tokens SET token_digest = $1")
        .bind(forged)
        .execute(&service.store.pool)
        .await
        .unwrap();
    assert_eq!(
        service.redeem("not-a-token", &browser, &clock).await.err(),
        Some(RecoveryError::Configuration)
    );
}

#[tokio::test]
async fn durability_capacity_and_close_are_enforced() {
    let (_directory, url, service) = service().await;
    let clock = Clock::at(T);
    for (weaken, restore) in [
        ("PRAGMA synchronous = NORMAL", "PRAGMA synchronous = FULL"),
        ("PRAGMA journal_mode = DELETE", "PRAGMA journal_mode = WAL"),
    ] {
        execute(&service, weaken).await;
        assert_eq!(
            service.purge_expired(&clock).await.err(),
            Some(RecoveryError::Configuration),
            "{weaken}"
        );
        execute(&service, restore).await;
        service.purge_expired(&clock).await.unwrap();
    }
    let other = service.clone();
    service.close().await;
    assert_eq!(
        other.purge_expired(&clock).await.err(),
        Some(RecoveryError::Storage)
    );
    let keys = RecoverySecrets::new([3; 32], [9; 32]).unwrap();
    let reopened = EmailLoginService::connect(&url, keys, config())
        .await
        .unwrap();
    execute(
        &reopened,
        "UPDATE rullst_email_login_control SET capacity = 5",
    )
    .await;
    assert_eq!(
        reopened.purge_expired(&clock).await.err(),
        Some(RecoveryError::Configuration)
    );
}
