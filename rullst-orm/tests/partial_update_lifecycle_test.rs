#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

mod partial_update_contract;

use rullst_orm::{Error, FromRow, Orm, Policy};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "partial_lessons", policy = "LessonPolicy")]
struct Lesson {
    id: i32,
    title: String,
    has_changes: i32,
    perform_update: String,
}
struct LessonPolicy;
#[rullst_orm::async_trait]
impl Policy<Lesson> for LessonPolicy {
    async fn can_update(model: &Lesson) -> Result<bool, Error> {
        Ok(model.title != "denied")
    }
}

#[tokio::test]
async fn rejected_partial_update_preserves_the_callers_model() {
    Orm::init_with_options("sqlite::memory:", 1, 30)
        .await
        .unwrap();
    sqlx::query("CREATE TABLE partial_lessons (id INTEGER PRIMARY KEY, title TEXT NOT NULL, has_changes INTEGER NOT NULL, perform_update TEXT NOT NULL)")
        .execute(Orm::pool().unwrap())
        .await
        .unwrap();
    let mut lesson = Lesson {
        id: 0,
        title: "original".into(),
        has_changes: 0,
        perform_update: "draft".into(),
    };
    lesson.save().await.unwrap();
    assert!(
        lesson
            .update_partial()
            .title("denied".into())
            .save()
            .await
            .is_err()
    );
    assert_eq!(
        Lesson::find(lesson.id).await.unwrap().unwrap().title,
        "original"
    );
    assert_eq!(
        lesson.title, "original",
        "denied input must not become local saved state"
    );
    // Existing column names must not collide with generated private helpers.
    lesson
        .update_partial()
        .has_changes(1)
        .perform_update("edited".into())
        .save()
        .await
        .unwrap();
    let saved = Lesson::find(lesson.id).await.unwrap().unwrap();
    assert_eq!(saved.has_changes, 1);
    assert_eq!(saved.perform_update, "edited");
    partial_update_contract::exercise().await;
}
