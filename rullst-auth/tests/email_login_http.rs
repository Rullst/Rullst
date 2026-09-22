#![cfg(any(feature = "email-login-sqlite", feature = "email-login-postgres"))]
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "email_login/http.rs"]
mod http;

#[cfg(feature = "email-login-sqlite")]
#[tokio::test]
#[ignore = "requires local Node and Chromium; explicitly exercised by the browser gate"]
async fn sqlite_email_login_browser() {
    let directory = tempfile::tempdir().unwrap();
    let url = http::support::sqlite_url(&directory.path().join("browser.db"));
    http::run(&url).await;
}

#[cfg(feature = "email-login-postgres")]
#[tokio::test]
#[ignore = "requires the wrapper-owned PostgreSQL, Node and Chromium"]
async fn postgres_email_login_browser() {
    let url = std::env::var("RULLST_RECOVERY_TEST_POSTGRES_URL").unwrap();
    assert_eq!(
        url::Url::parse(&url).unwrap().path(),
        "/rullst_recovery_contract"
    );
    http::run(&url).await;
}
