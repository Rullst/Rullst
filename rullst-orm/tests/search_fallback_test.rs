//! The generated SQL search fallback must not match hidden or protected
//! columns, and caller wildcards must match literally.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::{Error, FromRow, Orm, SecretString};

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "fallback_accounts", searchable)]
struct FallbackAccount {
    id: i32,
    name: String,
    #[orm(hidden)]
    reset_token: String,
    #[orm(masked)]
    recovery_hint: String,
    #[orm(encrypted)]
    note: String,
    cpf: SecretString,
}

async fn search_ids(query: &str) -> Result<Vec<i32>, Error> {
    FallbackAccount::search(query)
        .await
        .order_by("id")
        .pluck_i32("id")
        .await
}

#[tokio::test]
async fn sql_fallback_is_not_a_substring_oracle_for_protected_columns() {
    Orm::init_with_options(
        "sqlite:file:search_fallback_test.db?mode=memory&cache=shared",
        2,
        30,
    )
    .await
    .expect("initialize search fallback database");
    let pool = Orm::pool().expect("pool");
    for statement in [
        "CREATE TABLE fallback_accounts (id INTEGER PRIMARY KEY, name TEXT NOT NULL, reset_token TEXT NOT NULL, recovery_hint TEXT NOT NULL, note TEXT NOT NULL, cpf TEXT NOT NULL)",
        "INSERT INTO fallback_accounts VALUES \
            (1, 'alice', 'tok-7f3a9c', 'first-pet-rex', 'RULLST:v2:k:nonce:note', 'RULLST:v2:k:nonce:cpf'), \
            (2, 'bob 50% off', 'tok-000000', 'hint-000000', 'RULLST:v2:k:nonce:note', 'RULLST:v2:k:nonce:cpf'), \
            (3, 'carol_x', 'tok-111111', 'hint-111111', 'RULLST:v2:k:nonce:note', 'RULLST:v2:k:nonce:cpf')",
    ] {
        rullst_orm::_sqlx::query(statement)
            .execute(pool)
            .await
            .expect("seed search fallback fixture");
    }

    assert_eq!(search_ids("ali").await.unwrap(), vec![1]);
    for protected_probe in ["7f3a9c", "tok-", "first-pet", "RULLST", "nonce"] {
        assert!(
            search_ids(protected_probe).await.unwrap().is_empty(),
            "`{protected_probe}` matched a hidden or protected column"
        );
    }

    assert_eq!(search_ids("50%").await.unwrap(), vec![2]);
    assert_eq!(search_ids("%").await.unwrap(), vec![2]);
    assert!(search_ids("a_i").await.unwrap().is_empty());
    assert_eq!(search_ids("l_x").await.unwrap(), vec![3]);
    assert_eq!(search_ids("!").await.unwrap(), Vec::<i32>::new());

    let oversized = search_ids(&"a".repeat(1_025)).await;
    assert!(matches!(oversized, Err(Error::Validation(_))));
}
