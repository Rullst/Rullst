#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

// ORM initialization is process-global, so this contract has its own test binary.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, rullst_orm::Orm, rullst_orm::FromRow)]
#[orm(table = "optional_products")]
pub struct OptionalProduct {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
}

#[tokio::test]
async fn test_partial_updates_optional() {
    rullst_orm::Orm::init_with_options("sqlite::memory:", 1, 30)
        .await
        .unwrap();
    let pool = rullst_orm::Orm::pool().expect("ORM should be initialized");
    rullst_orm::_sqlx::query("CREATE TABLE optional_products (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, description TEXT)").execute(pool).await.unwrap();

    let mut p = OptionalProduct {
        id: 0,
        name: "A".to_string(),
        description: Some("desc".to_string()),
    };
    p.save().await.unwrap();

    // Update description to None
    p.update_partial().description(None).save().await.unwrap();

    let fetched = OptionalProduct::query()
        .where_eq("id", p.id)
        .first()
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.description, None);
}
