//! The registry contract on owned PostgreSQL and MySQL/MariaDB servers, run
//! by `.github/check-auth-session-registry.py`, which provides the URLs.
#![cfg(any(
    feature = "session-registry-postgres",
    feature = "session-registry-mysql"
))]
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "session_registry/contract.rs"]
#[allow(dead_code)]
mod contract;

#[cfg(feature = "session-registry-postgres")]
#[tokio::test]
#[ignore = "needs RULLST_SESSION_REGISTRY_POSTGRES_URL (.github/check-auth-session-registry.py)"]
async fn postgres_session_registry_contract() {
    use rullst_auth::session_registry::PostgresSessionRegistry;
    let url = std::env::var("RULLST_SESSION_REGISTRY_POSTGRES_URL").unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    sqlx::query("DROP TABLE IF EXISTS rullst_auth_sessions")
        .execute(&pool)
        .await
        .unwrap();
    // Two processes preparing the schema at once serialize.
    let (first, second) = tokio::join!(
        PostgresSessionRegistry::open(pool.clone()),
        PostgresSessionRegistry::open(pool.clone())
    );
    let registry = first.unwrap();
    second.unwrap();
    contract::run(&registry).await;
    let issued = registry
        .create_session(
            &rullst_auth::session_registry::NewSession::new(201, 0),
            contract::KEY,
            contract::NOW,
        )
        .await
        .unwrap();
    let (hash, public_id): (String, String) = sqlx::query_as(
        "SELECT token_hash, public_id FROM rullst_auth_sessions WHERE user_id = 201",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(hash.len(), 64);
    assert_eq!(public_id, issued.reference().as_str());
    assert!(!issued.token().contains(&hash));
}

#[cfg(feature = "session-registry-mysql")]
#[tokio::test]
#[ignore = "needs RULLST_SESSION_REGISTRY_MYSQL_URL (.github/check-auth-session-registry.py)"]
async fn mysql_session_registry_contract() {
    use rullst_auth::session_registry::{MySqlSessionRegistry, NewSession, SessionRef};
    let url = std::env::var("RULLST_SESSION_REGISTRY_MYSQL_URL").unwrap();
    let pool = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    sqlx::query("DROP TABLE IF EXISTS rullst_auth_sessions")
        .execute(&pool)
        .await
        .unwrap();
    let registry = MySqlSessionRegistry::open(pool.clone()).await.unwrap();
    MySqlSessionRegistry::open(pool.clone()).await.unwrap();
    contract::run(&registry).await;
    // References compare exactly, although MySQL's default collations
    // ignore case.
    let issued = registry
        .create_session(&NewSession::new(202, 0), contract::KEY, contract::NOW)
        .await
        .unwrap();
    let flipped: String = issued
        .reference()
        .as_str()
        .chars()
        .map(|character| {
            if character.is_ascii_lowercase() {
                character.to_ascii_uppercase()
            } else {
                character.to_ascii_lowercase()
            }
        })
        .collect();
    if flipped != issued.reference().as_str() {
        let other = SessionRef::parse(&flipped).unwrap();
        assert_eq!(registry.revoke_session(202, &other).await, Ok(false));
    }
    assert_eq!(
        registry.revoke_session(202, issued.reference()).await,
        Ok(true)
    );
}
