#![cfg(any(feature = "email-login-sqlite", feature = "email-login-postgres"))]
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "email_login/failures.rs"]
mod failures;
#[path = "email_login/lifecycle.rs"]
mod lifecycle;
#[path = "email_login/limits.rs"]
mod limits;
#[path = "email_login/locking.rs"]
mod locking;
#[cfg(feature = "email-login-postgres")]
#[path = "email_login/postgres.rs"]
mod postgres;
#[path = "email_login/support.rs"]
mod support;
use support::*;

#[cfg(feature = "email-login-sqlite")]
#[tokio::test]
async fn sqlite_login_contract() {
    let directory = tempfile::tempdir().unwrap();
    let url = sqlite_url(&directory.path().join("login 100%#.db"));
    lifecycle::run(&url).await;
    failures::run(&url).await;
    locking::run(&url).await;
    limits::run(&url).await;
}

#[cfg(feature = "email-login-postgres")]
#[tokio::test]
#[ignore = "requires the wrapper-owned disposable PostgreSQL"]
async fn postgres_login_contract() {
    let url =
        std::env::var("RULLST_RECOVERY_TEST_POSTGRES_URL").expect("owned PostgreSQL URL required");
    assert_eq!(
        url::Url::parse(&url).unwrap().path(),
        "/rullst_recovery_contract"
    );
    lifecycle::run(&url).await;
    failures::run(&url).await;
    locking::run(&url).await;
    limits::run(&url).await;
    postgres::run(&url).await;
}

#[test]
fn bounded_configuration_and_secret_debug() {
    for (landing, destination) in [
        ("http://app.example/login", "/"),
        ("https://app.example/login?q=1", "/"),
        ("https://user:pass@app.example/login", "/"),
        ("https://app.example/login", "//evil.example"),
        ("https://app.example/login", "/a/../b"),
        ("https://app.example/login", "/%2f/evil"),
        ("https://app.example/login", "/\\evil"),
    ] {
        assert!(EmailLoginConfig::new("school", landing, destination, 100).is_err());
    }
    assert!(EmailLoginConfig::for_development("school", "http://localhost/login", "/", 1).is_ok());
    assert!(
        EmailLoginConfig::for_development("school", "http://app.example/login", "/", 1).is_err()
    );
    assert!(EmailLoginConfig::new("bad:namespace", "https://app.example/login", "/", 1).is_err());
    assert!(EmailLoginConfig::new("school", "https://app.example/login", "/", 0).is_err());
    let binding = BrowserBinding::generate().unwrap();
    assert_eq!(binding.expose_cookie().len(), 43);
    assert!(!format!("{binding:?}").contains(binding.expose_cookie()));
    assert!(BrowserBinding::from_cookie("a").is_err());
    assert!(BrowserBinding::from_cookie(&"%".repeat(43)).is_err());
    assert_eq!(
        BrowserBinding::from_cookie(binding.expose_cookie())
            .unwrap()
            .expose_cookie(),
        binding.expose_cookie()
    );
}
