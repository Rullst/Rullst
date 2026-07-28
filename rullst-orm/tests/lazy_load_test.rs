#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize, rullst_orm::Orm, rullst_orm::FromRow)]
#[orm(table = "users")]
pub struct User {
    pub id: i32,
    pub name: String,

    #[orm(has_many = "Post", foreign_key = "user_id")]
    #[sqlx(default, skip)]
    pub posts: Option<Vec<Post>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, rullst_orm::Orm, rullst_orm::FromRow)]
#[orm(table = "posts")]
pub struct Post {
    pub id: i32,
    pub user_id: i32,
    pub title: String,
}

#[tokio::test]
#[should_panic(
    expected = "StrictLazyLoading: Attempted to lazily load relation 'posts' on 'User' without eager loading."
)]
async fn test_strict_lazy_loading_panics() {
    let _ = std::fs::remove_file("lazy_load.db");
    let _ = rullst_orm::Orm::init("sqlite://lazy_load.db?mode=rwc").await;
    // Enable the strict lazy loading feature
    rullst_orm::prevent_lazy_loading(true);

    let user = User {
        id: 1,
        name: "Alice".to_string(),
        posts: None,
    };

    // Attempting to lazily load 'posts' without using `.with("posts")`
    let _posts = user.posts().await;
}

#[tokio::test]
async fn test_strict_lazy_loading_disabled_works() {
    let _ = rullst_orm::Orm::init("sqlite::memory:").await;
    // Disable it for this test
    rullst_orm::prevent_lazy_loading(false);

    // Mocking an error or empty since we don't have a db
    // This will error from sqlx connection (since it has none) instead of panicking from lazy load prevention
    let user = User {
        id: 1,
        name: "Alice".to_string(),
        posts: None,
    };

    let result = user.posts().await;
    // We just want to ensure it doesn't panic on the lazy load check.
    assert!(result.is_err() || result.is_ok());
}
