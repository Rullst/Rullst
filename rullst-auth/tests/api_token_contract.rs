#![cfg(any(feature = "api-tokens-sqlite", feature = "api-tokens-postgres"))]
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "api_tokens/failures.rs"]
mod failures;
#[path = "api_tokens/lifecycle.rs"]
mod lifecycle;
#[cfg(feature = "api-tokens-postgres")]
#[path = "api_tokens/postgres.rs"]
mod postgres;
#[path = "api_tokens/support.rs"]
mod support;
use support::*;

#[cfg(feature = "api-tokens-sqlite")]
#[tokio::test]
async fn sqlite_api_token_contract() {
    let directory = tempfile::tempdir().unwrap();
    let url = sqlite_url(&directory.path().join("api 100%#.db"));
    lifecycle::run(&url).await;
    failures::run(&url).await;
}
#[cfg(feature = "api-tokens-postgres")]
#[tokio::test]
#[ignore = "requires wrapper-owned disposable PostgreSQL"]
async fn postgres_api_token_contract() {
    let url = std::env::var("RULLST_RECOVERY_TEST_POSTGRES_URL").unwrap();
    assert_eq!(
        url::Url::parse(&url).unwrap().path(),
        "/rullst_recovery_contract"
    );
    lifecycle::run(&url).await;
    failures::run(&url).await;
    postgres::run(&url).await;
}
#[test]
fn literal_bounded_scopes_and_configuration() {
    for scopes in [
        vec![],
        vec!["*"],
        vec!["orders:*"],
        vec!["READ"],
        vec![""],
        vec!["orders:read", "orders:read"],
        vec!["a,b"],
        vec!["a b"],
        vec!["álunos"],
    ] {
        assert!(ApiScopes::new(scopes).is_err());
    }
    assert!(ApiScopes::new(["x".repeat(65)]).is_err());
    assert!(ApiScopes::new((0..33).map(|i| format!("scope:{i}"))).is_err());
    let scopes = ApiScopes::new(["orders:write", "orders:read"]).unwrap();
    assert_eq!(
        scopes.iter().collect::<Vec<_>>(),
        vec!["orders:read", "orders:write"]
    );
    for (namespace, capacity, lifetime) in [
        ("", 1, 1),
        ("a:b", 1, 1),
        ("a", 0, 1),
        ("a", 100001, 1),
        ("a", 1, 0),
        ("a", 1, 30 * 86400 + 1),
    ] {
        assert!(ApiTokenConfig::new(namespace, scopes.clone(), capacity, lifetime).is_err());
    }
    assert!(ApiTokenId::new("secret").is_err());
}
