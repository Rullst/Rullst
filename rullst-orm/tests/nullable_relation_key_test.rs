//! Relations whose key is a nullable foreign key (`Option<i32>`) compile and
//! load: a `None` key matches no row on either side of the relation.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::{FromRow, Orm};

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "nullable_authors")]
struct NullableAuthor {
    id: i32,
    name: String,
    mentor_id: Option<i32>,
    /// Child foreign key is nullable.
    #[orm(has_many = "NullablePost", foreign_key = "author_id")]
    #[sqlx(default, skip)]
    posts: Option<Vec<NullablePost>>,
    #[orm(has_one = "NullableProfile", foreign_key = "author_id")]
    #[sqlx(default, skip)]
    profile: Option<NullableProfile>,
    /// Both the local key on this model and the child key are nullable.
    #[orm(
        has_many = "NullablePost",
        foreign_key = "author_id",
        local_key = "mentor_id"
    )]
    #[sqlx(default, skip)]
    mentor_posts: Option<Vec<NullablePost>>,
}

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "nullable_posts")]
struct NullablePost {
    id: i32,
    author_id: Option<i32>,
    title: String,
    /// The foreign key on this model is nullable.
    #[orm(belongs_to = "NullableAuthor", foreign_key = "author_id")]
    #[sqlx(default, skip)]
    author: Option<NullableAuthor>,
}

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "nullable_profiles")]
struct NullableProfile {
    id: i32,
    author_id: Option<i32>,
    bio: String,
}

fn post_ids(posts: Option<&Vec<NullablePost>>) -> Option<Vec<i32>> {
    posts.map(|posts| posts.iter().map(|post| post.id).collect())
}

async fn setup() {
    Orm::init_with_options("sqlite::memory:", 1, 5)
        .await
        .expect("initialize SQLite");
    let pool = Orm::pool().expect("pool");
    for statement in [
        "CREATE TABLE nullable_authors (id INTEGER PRIMARY KEY, name TEXT NOT NULL, mentor_id INTEGER)",
        "CREATE TABLE nullable_posts (id INTEGER PRIMARY KEY, author_id INTEGER, title TEXT NOT NULL)",
        "CREATE TABLE nullable_profiles (id INTEGER PRIMARY KEY, author_id INTEGER, bio TEXT NOT NULL)",
        "INSERT INTO nullable_authors (id, name, mentor_id) VALUES (1, 'ada', NULL), (2, 'grace', 1), (3, 'linus', NULL)",
        "INSERT INTO nullable_posts (id, author_id, title) VALUES (1, 1, 'a1'), (2, NULL, 'orphan'), (3, 1, 'a2'), (4, 2, 'g1')",
        "INSERT INTO nullable_profiles (id, author_id, bio) VALUES (1, NULL, 'orphan'), (2, 2, 'grace')",
    ] {
        rullst_orm::_sqlx::query(statement)
            .execute(pool)
            .await
            .expect("schema/seed statement");
    }
}

async fn belongs_to_skips_a_missing_parent() {
    let orphan = NullablePost::find(2).await.unwrap().expect("orphan post");
    assert!(orphan.author().await.expect("lazy null key").is_none());
    let post = NullablePost::find(4).await.unwrap().expect("post");
    let author = post.author().await.expect("lazy key").expect("author");
    assert_eq!(author.name, "grace");

    let posts = NullablePost::query()
        .order_by("id")
        .with_author()
        .get()
        .await
        .expect("eager belongs_to");
    let authors = posts
        .iter()
        .map(|post| post.author.as_ref().map(|author| author.id))
        .collect::<Vec<_>>();
    assert_eq!(authors, [Some(1), None, Some(1), Some(2)]);

    let orphans = NullablePost::query()
        .where_null("author_id")
        .with_author()
        .get()
        .await
        .expect("eager belongs_to without any parent key");
    assert_eq!(orphans.len(), 1);
    assert!(orphans[0].author.is_none());
}

async fn has_relations_ignore_children_without_a_parent() {
    let ada = NullableAuthor::find(1).await.unwrap().expect("ada");
    assert_eq!(
        post_ids(Some(&ada.posts().await.expect("lazy has_many"))),
        Some(vec![1, 3])
    );
    assert!(ada.profile().await.expect("lazy has_one").is_none());
    // `mentor_id` is NULL: nothing to load, and no row with a NULL key matches.
    assert!(ada.mentor_posts().await.expect("lazy null key").is_empty());
    let grace = NullableAuthor::find(2).await.unwrap().expect("grace");
    assert_eq!(
        post_ids(Some(&grace.mentor_posts().await.expect("lazy key"))),
        Some(vec![1, 3])
    );

    let authors = NullableAuthor::query()
        .order_by("id")
        .with_posts()
        .with_profile()
        .with_mentor_posts()
        .get()
        .await
        .expect("eager has_many/has_one");
    let posts = authors
        .iter()
        .map(|author| post_ids(author.posts.as_ref()))
        .collect::<Vec<_>>();
    assert_eq!(posts, [Some(vec![1, 3]), Some(vec![4]), Some(Vec::new())]);
    let profiles = authors
        .iter()
        .map(|author| author.profile.as_ref().map(|profile| profile.id))
        .collect::<Vec<_>>();
    assert_eq!(profiles, [None, Some(2), None]);
    let mentor_posts = authors
        .iter()
        .map(|author| post_ids(author.mentor_posts.as_ref()))
        .collect::<Vec<_>>();
    assert_eq!(
        mentor_posts,
        [Some(Vec::new()), Some(vec![1, 3]), Some(Vec::new())]
    );
}

#[tokio::test]
async fn nullable_relation_keys_load_without_matching_null() {
    setup().await;
    belongs_to_skips_a_missing_parent().await;
    has_relations_ignore_children_without_a_parent().await;
}
