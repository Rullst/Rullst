#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

use rullst_orm::{FromRow, Orm};

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "morph_posts")]
struct MorphPost {
    id: i32,
    title: String,
}

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "morph_videos")]
struct MorphVideo {
    id: i32,
    title: String,
}

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "morph_comments")]
struct MorphComment {
    id: i32,
    body: String,
    commentable_id: i32,
    commentable_type: String,
    #[orm(morph_to = "MorphPost", morph_name = "commentable")]
    #[sqlx(default, skip)]
    post: Option<MorphPost>,
    #[orm(morph_to = "MorphVideo", morph_name = "commentable")]
    #[sqlx(default, skip)]
    video: Option<MorphVideo>,
}

#[tokio::test]
async fn morph_to_loads_only_the_discriminated_target_and_batches_duplicates() {
    let database_path = "morph_to_test.db";
    let _ = std::fs::remove_file(database_path);
    Orm::init(&format!("sqlite:{database_path}?mode=rwc"))
        .await
        .expect("initialize morph-to database");
    let pool = Orm::try_pool().expect("ORM pool");

    for statement in [
        "CREATE TABLE morph_posts (id INTEGER PRIMARY KEY, title TEXT NOT NULL)",
        "CREATE TABLE morph_videos (id INTEGER PRIMARY KEY, title TEXT NOT NULL)",
        "CREATE TABLE morph_comments (id INTEGER PRIMARY KEY, body TEXT NOT NULL, commentable_id INTEGER NOT NULL, commentable_type TEXT NOT NULL)",
        "INSERT INTO morph_posts (id, title) VALUES (1, 'Typed post')",
        "INSERT INTO morph_videos (id, title) VALUES (1, 'Typed video')",
        "INSERT INTO morph_comments (id, body, commentable_id, commentable_type) VALUES (1, 'post one', 1, 'MorphPost')",
        "INSERT INTO morph_comments (id, body, commentable_id, commentable_type) VALUES (2, 'video one', 1, 'MorphVideo')",
        "INSERT INTO morph_comments (id, body, commentable_id, commentable_type) VALUES (3, 'post duplicate', 1, 'MorphPost')",
    ] {
        sqlx::query(statement)
            .execute(pool)
            .await
            .expect("prepare morph-to fixture");
    }

    let post_comment = MorphComment::query()
        .where_id(1)
        .first()
        .await
        .expect("load post comment")
        .expect("post comment exists");
    assert_eq!(
        post_comment
            .post()
            .await
            .expect("lazy post relation")
            .expect("post target")
            .title,
        "Typed post"
    );
    assert!(
        post_comment
            .video()
            .await
            .expect("mismatched lazy target")
            .is_none()
    );

    let comments = MorphComment::query()
        .with_post()
        .with_video()
        .order_by_id()
        .get()
        .await
        .expect("batch eager-load polymorphic targets");
    assert_eq!(comments.len(), 3);
    assert_eq!(comments[0].body, "post one");
    assert_eq!(
        comments[0].post.as_ref().map(|post| post.title.as_str()),
        Some("Typed post")
    );
    assert!(comments[0].video.is_none());
    assert_eq!(
        comments[1].video.as_ref().map(|video| video.title.as_str()),
        Some("Typed video")
    );
    assert!(comments[1].post.is_none());
    assert_eq!(
        comments[2].post.as_ref().map(|post| post.title.as_str()),
        Some("Typed post")
    );

    pool.close().await;
    std::fs::remove_file(database_path).expect("remove morph-to database");
}
