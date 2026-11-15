//! A migration owns the table; revocations follow the caller's transaction.

use super::*;

#[tokio::test]
async fn a_migration_can_own_the_table() {
    let pool = pool().await;
    // Without the table, `connect` creates nothing and fails closed.
    assert_eq!(
        SqliteSessionRegistry::connect(pool.clone()).await.err(),
        Some(SessionRegistryError::Unavailable)
    );
    SqliteSessionRegistry::create_table(&pool).await.unwrap();
    // Creating it again keeps it and its rows.
    let registry = SqliteSessionRegistry::connect(pool.clone()).await.unwrap();
    let issued = login(&registry, 1, NOW).await;
    SqliteSessionRegistry::create_table(&pool).await.unwrap();
    assert_eq!(rows(&pool).await.len(), 1);
    assert!(
        registry
            .authenticate(issued.token(), KEY, NOW)
            .await
            .is_ok()
    );
    // The rollback of such a migration.
    execute(&pool, "DROP TABLE rullst_auth_sessions").await;
    assert_eq!(
        SqliteSessionRegistry::connect(pool.clone()).await.err(),
        Some(SessionRegistryError::Unavailable)
    );
    assert_eq!(
        registry.authenticate(issued.token(), KEY, NOW).await.err(),
        Some(SessionRegistryError::Unavailable)
    );
}

#[tokio::test]
async fn revoking_inside_a_transaction_follows_it() {
    let (registry, pool) = registry().await;
    let first = login(&registry, 3, NOW).await;
    let second = login(&registry, 3, NOW).await;
    let other = login(&registry, 4, NOW).await;
    let mut transaction = pool.begin().await.unwrap();
    assert_eq!(
        SqliteSessionRegistry::revoke_all_sessions_on(&mut transaction, 3).await,
        Ok(2)
    );
    transaction.rollback().await.unwrap();
    assert!(registry.authenticate(first.token(), KEY, NOW).await.is_ok());
    let mut transaction = pool.begin().await.unwrap();
    assert_eq!(
        SqliteSessionRegistry::revoke_all_sessions_on(&mut transaction, 3).await,
        Ok(2)
    );
    transaction.commit().await.unwrap();
    for revoked in [&first, &second] {
        assert_eq!(
            registry.authenticate(revoked.token(), KEY, NOW).await.err(),
            Some(SessionRegistryError::Revoked)
        );
    }
    assert!(registry.authenticate(other.token(), KEY, NOW).await.is_ok());
    let mut connection = pool.acquire().await.unwrap();
    assert_eq!(
        SqliteSessionRegistry::revoke_all_sessions_on(&mut connection, 3).await,
        Ok(0)
    );
}
