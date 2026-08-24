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
async fn test_strict_lazy_loading_returns_typed_error() {
    rullst_orm::Orm::init("sqlite::memory:").await.unwrap();
    rullst_orm::prevent_lazy_loading(true);

    let user = User {
        id: 1,
        name: "Alice".to_string(),
        posts: None,
    };

    let error = user.posts().await.unwrap_err();
    assert!(matches!(error, rullst_orm::Error::Validation(_)));
    assert!(error.to_string().contains(
        "StrictLazyLoading: attempted to lazily load relation 'posts' on 'User' without eager loading"
    ));

    rullst_orm::prevent_lazy_loading(false);
    let result = user.posts().await;
    assert!(
        !matches!(result, Err(rullst_orm::Error::Validation(message)) if message.starts_with("StrictLazyLoading:"))
    );
}
