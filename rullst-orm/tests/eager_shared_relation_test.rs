//! Eager loading gives every parent the relation it shares with other parents.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::{Error, FromRow, Orm};

#[derive(Clone, Debug, PartialEq, FromRow, rullst_orm::Orm)]
#[orm(table = "shared_users")]
struct SharedUser {
    id: i32,
    name: String,
}

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "shared_posts")]
struct SharedPost {
    id: i32,
    user_id: i32,
    title: String,
    #[orm(belongs_to = "SharedUser", foreign_key = "user_id")]
    #[sqlx(default, skip)]
    author: Option<SharedUser>,
}

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "shared_members")]
struct SharedMember {
    id: i32,
    organization_id: i32,
    name: String,
    #[orm(
        has_many = "SharedNotice",
        foreign_key = "organization_id",
        local_key = "organization_id"
    )]
    #[sqlx(default, skip)]
    notices: Option<Vec<SharedNotice>>,
}

#[derive(Clone, Debug, PartialEq, FromRow, rullst_orm::Orm)]
#[orm(table = "shared_notices")]
struct SharedNotice {
    id: i32,
    organization_id: i32,
    body: String,
}

/// Deliberately not `Clone`: sharing it among parents must fail closed.
#[derive(Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "shared_badges")]
struct SharedBadge {
    id: i32,
    label: String,
}

#[derive(Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "shared_holders")]
struct SharedHolder {
    id: i32,
    badge_id: i32,
    #[orm(belongs_to = "SharedBadge", foreign_key = "badge_id")]
    #[sqlx(default, skip)]
    badge: Option<SharedBadge>,
}

async fn setup() {
    Orm::init_with_options("sqlite::memory:", 1, 5)
        .await
        .expect("initialize SQLite");
    let pool = Orm::pool().expect("pool");
    for statement in [
        "CREATE TABLE shared_users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
        "CREATE TABLE shared_posts (id INTEGER PRIMARY KEY, user_id INTEGER NOT NULL, title TEXT NOT NULL)",
        "CREATE TABLE shared_members (id INTEGER PRIMARY KEY, organization_id INTEGER NOT NULL, name TEXT NOT NULL)",
        "CREATE TABLE shared_notices (id INTEGER PRIMARY KEY, organization_id INTEGER NOT NULL, body TEXT NOT NULL)",
        "CREATE TABLE shared_badges (id INTEGER PRIMARY KEY, label TEXT NOT NULL)",
        "CREATE TABLE shared_holders (id INTEGER PRIMARY KEY, badge_id INTEGER NOT NULL)",
        "INSERT INTO shared_users (id, name) VALUES (7, 'ada'), (8, 'grace')",
        "INSERT INTO shared_posts (id, user_id, title) VALUES (1, 7, 'a'), (2, 7, 'b'), (3, 8, 'c'), (4, 7, 'd')",
        "INSERT INTO shared_members (id, organization_id, name) VALUES (1, 10, 'x'), (2, 10, 'y'), (3, 20, 'z'), (4, 30, 'w')",
        "INSERT INTO shared_notices (id, organization_id, body) VALUES (1, 10, 'n1'), (2, 10, 'n2'), (3, 20, 'n3')",
        "INSERT INTO shared_badges (id, label) VALUES (1, 'gold'), (2, 'silver')",
        "INSERT INTO shared_holders (id, badge_id) VALUES (1, 1), (2, 2)",
    ] {
        rullst_orm::_sqlx::query(statement)
            .execute(pool)
            .await
            .expect("schema/seed statement");
    }
}

#[tokio::test]
async fn shared_relations_reach_every_parent_or_fail_closed() {
    setup().await;

    // belongs_to: three posts share author 7; each one gets it.
    let posts = SharedPost::query()
        .with_author()
        .order_by("id")
        .get()
        .await
        .expect("eager belongs_to");
    let authors = posts
        .iter()
        .map(|post| post.author.as_ref().map(|user| user.name.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(
        authors,
        [Some("ada"), Some("ada"), Some("grace"), Some("ada")]
    );

    // has_many over a non-unique local key: members of one organization all
    // receive its notices, and a key without rows yields an empty list.
    let members = SharedMember::query()
        .with_notices()
        .order_by("id")
        .get()
        .await
        .expect("eager has_many");
    let notice_counts = members
        .iter()
        .map(|member| member.notices.as_ref().map(Vec::len))
        .collect::<Vec<_>>();
    assert_eq!(notice_counts, [Some(2), Some(2), Some(1), Some(0)]);
    assert_eq!(members[0].notices, members[1].notices);

    // A non-Clone related model still loads when no row is shared...
    let holders = SharedHolder::query()
        .with_badge()
        .order_by("id")
        .get()
        .await
        .expect("unshared non-Clone relation");
    assert_eq!(
        holders[0].badge.as_ref().map(|b| b.label.as_str()),
        Some("gold")
    );
    assert_eq!(
        holders[1].badge.as_ref().map(|b| b.label.as_str()),
        Some("silver")
    );

    // ...but sharing one row among several parents fails instead of leaving
    // later parents without their relation.
    rullst_orm::_sqlx::query("INSERT INTO shared_holders (id, badge_id) VALUES (3, 1)")
        .execute(Orm::pool().expect("pool"))
        .await
        .expect("seed shared holder");
    match SharedHolder::query().with_badge().get().await {
        Err(Error::Validation(message)) => {
            assert!(message.contains("must implement Clone"), "{message}");
        }
        other => panic!("sharing a non-Clone relation must fail closed: {other:?}"),
    }
}
