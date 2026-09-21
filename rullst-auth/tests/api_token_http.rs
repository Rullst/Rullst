#![cfg(any(feature = "api-tokens-sqlite", feature = "api-tokens-postgres"))]
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "api_tokens/http.rs"]
mod http;

#[cfg(feature = "api-tokens-sqlite")]
#[tokio::test]
async fn sqlite_api_tokens_http_authorization_and_revocation() {
    let directory = tempfile::tempdir().unwrap();
    let url = http::support::sqlite_url(&directory.path().join("http.db"));
    http::run(&url).await;
}
#[cfg(feature = "api-tokens-postgres")]
#[tokio::test]
#[ignore = "requires wrapper-owned PostgreSQL"]
async fn postgres_api_tokens_http_authorization_and_revocation() {
    let url = std::env::var("RULLST_RECOVERY_TEST_POSTGRES_URL").unwrap();
    assert_eq!(
        url::Url::parse(&url).unwrap().path(),
        "/rullst_recovery_contract"
    );
    http::run(&url).await;
}
