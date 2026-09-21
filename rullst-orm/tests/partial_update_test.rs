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
    rullst_orm::Orm::init_with_options("sqlite::memory:", 1, 30)
        .await
        .unwrap();

    // Create a table manually
    let pool = rullst_orm::Orm::pool().expect("ORM should be initialized");
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
