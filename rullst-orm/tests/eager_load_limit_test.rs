//! Eager loading never assigns relations from a query truncated by the global
//! row cap. This binary owns its process-global ORM and a cap of five rows.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::{Error, FromRow, Orm};

const CAP: usize = 5;

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "eager_posts")]
struct EagerPost {
    id: i32,
    title: String,
    #[orm(has_many = "EagerComment", foreign_key = "post_id")]
    #[sqlx(default, skip)]
    comments: Option<Vec<EagerComment>>,
    #[orm(has_one = "EagerComment", foreign_key = "post_id")]
    #[sqlx(default, skip)]
    first_comment: Option<EagerComment>,
    #[orm(morph_many = "EagerImage", morph_name = "imageable")]
    #[sqlx(default, skip)]
    images: Option<Vec<EagerImage>>,
    #[orm(
        belongs_to_many = "EagerTag",
        pivot_table = "eager_post_tags",
        foreign_key = "post_id",
        related_key = "tag_id"
    )]
    #[sqlx(default, skip)]
    tags: Option<Vec<EagerTag>>,
}

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "eager_comments")]
struct EagerComment {
    id: i32,
    post_id: i32,
    body: String,
}

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "eager_images")]
struct EagerImage {
    id: i32,
    imageable_id: i32,
    imageable_type: String,
    url: String,
}

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "eager_tags")]
struct EagerTag {
    id: i32,
    name: String,
}

fn assert_limit_error<T: std::fmt::Debug>(result: Result<T, Error>, relation: &str) {
    match result {
        Err(Error::Validation(message)) => {
            assert!(
                message.contains(&format!("`{relation}` for `EagerPost`")),
                "{message}"
            );
            assert!(message.contains("5-row query limit"), "{message}");
        }
        other => panic!("eager loading `{relation}` must fail closed, got {other:?}"),
    }
}

fn comment_counts(posts: &[EagerPost]) -> Vec<(i32, usize)> {
    posts
        .iter()
        .map(|post| {
            let comments = post.comments.as_ref().expect("comments were eager loaded");
            assert!(comments.iter().all(|comment| comment.post_id == post.id));
            (post.id, comments.len())
        })
        .collect()
}

async fn seed() {
    let pool = Orm::pool().expect("SQLite pool");
    for statement in [
        "CREATE TABLE eager_posts (id INTEGER PRIMARY KEY, title TEXT NOT NULL)",
        "CREATE TABLE eager_comments (id INTEGER PRIMARY KEY, post_id INTEGER NOT NULL, body TEXT NOT NULL)",
        "CREATE TABLE eager_images (id INTEGER PRIMARY KEY, imageable_id INTEGER NOT NULL, imageable_type TEXT NOT NULL, url TEXT NOT NULL)",
        "CREATE TABLE eager_tags (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
        "CREATE TABLE eager_post_tags (post_id INTEGER NOT NULL, tag_id INTEGER NOT NULL)",
        "INSERT INTO eager_posts (id, title) VALUES (1, 'three'), (2, 'two'), (3, 'four')",
        // 3 + 2 comments for posts 1 and 2 fill the cap exactly; post 3 adds 4.
        "INSERT INTO eager_comments (id, post_id, body) VALUES \
            (1, 1, 'a'), (2, 1, 'b'), (3, 1, 'c'), (4, 2, 'd'), (5, 2, 'e'), \
            (6, 3, 'f'), (7, 3, 'g'), (8, 3, 'h'), (9, 3, 'i')",
        "INSERT INTO eager_images (id, imageable_id, imageable_type, url) VALUES \
            (1, 1, 'EagerPost', 'a'), (2, 1, 'EagerPost', 'b'), (3, 1, 'EagerPost', 'c'), \
            (4, 1, 'EagerPost', 'd'), (5, 1, 'EagerPost', 'e'), (6, 2, 'EagerPost', 'f')",
        "INSERT INTO eager_tags (id, name) VALUES (1, 'a'), (2, 'b'), (3, 'c'), (4, 'd'), (5, 'e'), (6, 'f')",
        "INSERT INTO eager_post_tags (post_id, tag_id) VALUES \
            (1, 1), (1, 2), (1, 3), (1, 4), (1, 5), (2, 6)",
    ] {
        sqlx::query(statement)
            .execute(pool)
            .await
            .expect("seed eager-load fixture");
    }
}

#[tokio::test]
async fn eager_loads_fail_closed_instead_of_truncating_at_the_query_limit() {
    let database_path = std::env::temp_dir().join(format!(
        "rullst-eager-limit-{}-{}.db",
        std::process::id(),
        rand::random::<u64>()
    ));
    Orm::init(&format!("sqlite:{}?mode=rwc", database_path.display()))
        .await
        .expect("initialize SQLite eager-load database");
    seed().await;
    Orm::set_max_query_limit(CAP);

    // Exactly at the cap: every related row is loaded and assigned.
    let within_cap = EagerPost::query()
        .where_in("id", vec![1, 2])
        .order_by("id")
        .with_comments()
        .get()
        .await
        .expect("five related rows fit the cap");
    assert_eq!(comment_counts(&within_cap), vec![(1, 3), (2, 2)]);

    // Nine related rows across three parents: the old single capped query
    // silently gave some parents a partial or empty list.
    assert_limit_error(EagerPost::query().with_comments().get().await, "comments");
    assert_limit_error(
        EagerPost::query().with_first_comment().get().await,
        "first_comment",
    );

    // An explicit constraint remains the caller's decision.
    let limited = EagerPost::query()
        .order_by("id")
        .with_comments_constrained(|comments| comments.order_by("id").limit(2))
        .get()
        .await
        .expect("an explicit smaller relation limit is honored");
    assert_eq!(
        limited
            .iter()
            .map(|post| post.comments.as_ref().map_or(0, Vec::len))
            .sum::<usize>(),
        2
    );
    let unlimited = EagerPost::query()
        .order_by("id")
        .with_comments_constrained(|comments| comments.unsafe_unlimited())
        .get()
        .await
        .expect("an explicit unsafe_unlimited relation loads everything");
    assert_eq!(comment_counts(&unlimited), vec![(1, 3), (2, 2), (3, 4)]);

    // morph_many and belongs_to_many share the same guard.
    let images = EagerPost::query()
        .where_id(1)
        .with_images()
        .get()
        .await
        .expect("five polymorphic rows fit the cap");
    assert_eq!(images[0].images.as_ref().map(Vec::len), Some(5));
    assert_limit_error(EagerPost::query().with_images().get().await, "images");
    let tags = EagerPost::query()
        .where_id(1)
        .with_tags()
        .get()
        .await
        .expect("five pivot targets fit the cap");
    assert_eq!(tags[0].tags.as_ref().map(Vec::len), Some(5));
    assert_limit_error(EagerPost::query().with_tags().get().await, "tags");

    // A disabled cap never guards.
    Orm::set_max_query_limit(0);
    let everything = EagerPost::query()
        .order_by("id")
        .with_comments()
        .get()
        .await
        .expect("a disabled cap loads every related row");
    assert_eq!(comment_counts(&everything), vec![(1, 3), (2, 2), (3, 4)]);

    Orm::pool().expect("SQLite pool").close().await;
    let _ = std::fs::remove_file(database_path);
}
