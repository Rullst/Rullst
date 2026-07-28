#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize, rullst_orm::Orm, rullst_orm::FromRow)]
#[orm(table = "products")]
pub struct Product {
    pub id: i32,
    pub name: String,
    pub price: i32,
    pub description: String,
}

#[tokio::test]
async fn test_partial_updates() {
    let _ = std::fs::remove_file("partial_update.db");
    let _ = rullst_orm::Orm::init("sqlite://partial_update.db?mode=rwc").await;

    // Create a table manually
    let pool = rullst_orm::Orm::pool();
    rullst_orm::_sqlx::query(
        "CREATE TABLE products (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            price INTEGER NOT NULL,
            description TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .unwrap();

    // Insert a product
    let mut product = Product {
        id: 0,
        name: "Old Name".to_string(),
        price: 100,
        description: "Old Desc".to_string(),
    };
    product.save().await.unwrap();
    assert_eq!(product.id, 1);

    // Update partially
    product
        .update_partial()
        .name("New Name".to_string())
        .price(200)
        .save()
        .await
        .unwrap();

    assert_eq!(product.name, "New Name");
    assert_eq!(product.price, 200);
    assert_eq!(product.description, "Old Desc"); // Remains unchanged in memory

    // Verify database state
    let mut fetched = Product::query()
        .where_eq("id", 1)
        .first()
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.name, "New Name");
    assert_eq!(fetched.price, 200);
    assert_eq!(fetched.description, "Old Desc");

    // Partial update with no fields should be a no-op
    fetched.update_partial().save().await.unwrap();

    let fetched2 = Product::query()
        .where_eq("id", 1)
        .first()
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched2.name, "New Name");
}

#[derive(Clone, Debug, Serialize, Deserialize, rullst_orm::Orm, rullst_orm::FromRow)]
#[orm(table = "optional_products")]
pub struct OptionalProduct {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
}

#[tokio::test]
async fn test_partial_updates_optional() {
    let _ = std::fs::remove_file("partial_update_opt.db");
    let _ = rullst_orm::Orm::init("sqlite://partial_update_opt.db?mode=rwc").await;
    let pool = rullst_orm::Orm::pool();
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
