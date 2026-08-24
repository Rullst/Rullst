#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

use rullst_orm::Policy;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, rullst_orm::Orm, rullst_orm::FromRow)]
#[orm(table = "documents", policy = "DocumentPolicy")]
pub struct Document {
    pub id: i32,
    pub title: String,
    pub user_id: i32,
}

pub struct DocumentPolicy;

#[async_trait::async_trait]
impl Policy<Document> for DocumentPolicy {
    async fn can_create(model: &Document) -> Result<bool, rullst_orm::Error> {
        // Only allow creating if user_id is 1
        Ok(model.user_id == 1)
    }

    async fn can_update(model: &Document) -> Result<bool, rullst_orm::Error> {
        // Prevent update if title is "locked"
        Ok(model.title != "locked")
    }

    async fn can_delete(model: &Document) -> Result<bool, rullst_orm::Error> {
        Ok(model.user_id == 1)
    }
}

#[tokio::test]
async fn test_policy_enforcement() {
    let _ = std::fs::remove_file("policy.db");
    let _ = rullst_orm::Orm::init("sqlite://policy.db?mode=rwc").await;

    // Create a table manually
    let pool = rullst_orm::Orm::pool().expect("ORM should be initialized");
    rullst_orm::_sqlx::query(
        "CREATE TABLE documents (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            user_id INTEGER NOT NULL
        )",
    )
    .execute(pool)
    .await
    .unwrap();

    // 1. Creating - should fail
    let mut doc_fail = Document {
        id: 0,
        title: "Test".to_string(),
        user_id: 2, // policy allows only 1
    };
    let res = doc_fail.save().await;
    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err().to_string(),
        "Validation error: Policy prevents creation of this record"
    );

    // 2. Creating - should succeed
    let mut doc_ok = Document {
        id: 0,
        title: "Test".to_string(),
        user_id: 1,
    };
    doc_ok.save().await.unwrap();

    // 3. Updating - should fail
    doc_ok.title = "locked".to_string();
    let res = doc_ok.save().await;
    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err().to_string(),
        "Validation error: Policy prevents updating this record"
    );

    // 4. Updating - should succeed
    doc_ok.title = "unlocked".to_string();
    doc_ok.save().await.unwrap();

    // 5. Deleting - should fail
    doc_ok.user_id = 2; // change user id to test deletion policy
    let res = doc_ok.delete().await;
    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err().to_string(),
        "Validation error: Policy prevents deleting this record"
    );

    // 6. Deleting - should succeed
    doc_ok.user_id = 1;
    doc_ok.delete().await.unwrap();
}

pub struct DefaultPolicy;
#[async_trait::async_trait]
impl Policy<Document> for DefaultPolicy {}

#[tokio::test]
async fn test_default_policy() {
    let doc = Document {
        id: 0,
        title: "A".to_string(),
        user_id: 1,
    };
    assert!(DefaultPolicy::can_create(&doc).await.unwrap());
    assert!(DefaultPolicy::can_update(&doc).await.unwrap());
    assert!(DefaultPolicy::can_delete(&doc).await.unwrap());
    assert!(DefaultPolicy::can_restore(&doc).await.unwrap());
    assert!(DefaultPolicy::can_force_delete(&doc).await.unwrap());
}
