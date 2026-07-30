#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

use rullst_orm::Orm;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Orm, sqlx::FromRow, Serialize, Deserialize, PartialEq)]
#[orm(table = "cte_categories")]
pub struct Category {
    pub id: i32,
    pub parent_id: Option<i32>,
    pub name: String,
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_cte_graph_traversal() {
    // 1. Initialize SQLite in-memory database
    Orm::init_with_options("sqlite:file:cte_test_db_2?mode=memory&cache=shared", 5, 30)
        .await
        .expect("Failed to init ORM in test");
    let pool = Orm::pool();

    // 2. Create the table schema
    sqlx::query(
        "CREATE TABLE cte_categories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            parent_id INTEGER,
            name TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .unwrap();

    // 3. Seed nested data (Root -> A -> B -> C)
    let mut root = Category {
        id: 0,
        parent_id: None,
        name: "Root".to_string(),
    };
    root.save().await.unwrap();
    let mut a = Category {
        id: 0,
        parent_id: Some(root.id),
        name: "A".to_string(),
    };
    a.save().await.unwrap();
    let mut b = Category {
        id: 0,
        parent_id: Some(a.id),
        name: "B".to_string(),
    };
    b.save().await.unwrap();
    let mut c = Category {
        id: 0,
        parent_id: Some(b.id),
        name: "C".to_string(),
    };
    c.save().await.unwrap();

    // 4. Test WITH RECURSIVE via raw queries built with `.with_recursive_raw()`
    let recursive_sql = format!(
        "SELECT id, parent_id, name FROM cte_categories WHERE id = {}
         UNION ALL
         SELECT c.id, c.parent_id, c.name FROM cte_categories c
         INNER JOIN category_tree ct ON c.parent_id = ct.id",
        root.id
    );

    let query = Category::query().with_recursive_raw("category_tree", &recursive_sql);

    // Check if the SQL is generated correctly
    let generated_sql = query.to_sql();
    assert!(generated_sql.contains("WITH RECURSIVE category_tree AS"));

    let query_joined = query.join(
        "category_tree",
        "cte_categories.id",
        "=",
        "category_tree.id",
    );
    let results = query_joined.get().await.unwrap();

    // The results should contain all 4 categories (Root, A, B, C) because we recursively walked the tree
    assert_eq!(results.len(), 4);
    assert!(results.iter().any(|cat| cat.name == "Root"));
    assert!(results.iter().any(|cat| cat.name == "A"));
    assert!(results.iter().any(|cat| cat.name == "B"));
    assert!(results.iter().any(|cat| cat.name == "C"));
}
