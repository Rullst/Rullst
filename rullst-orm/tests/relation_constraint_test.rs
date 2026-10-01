//! Generated relation queries keep their ownership predicate and defaults.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::sync::Arc;

use rullst_orm::{FromRow, Orm};

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "constraint_accounts")]
struct ConstraintAccount {
    id: i32,
    name: String,
    #[orm(has_many = "ConstraintDocument", foreign_key = "account_id")]
    #[sqlx(default, skip)]
    documents: Option<Vec<ConstraintDocument>>,
    #[orm(
        belongs_to_many = "ConstraintTag",
        pivot_table = "constraint_account_tags",
        foreign_key = "account_id",
        related_key = "tag_id"
    )]
    #[sqlx(default, skip)]
    tags: Option<Vec<ConstraintTag>>,
}

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "constraint_tags")]
struct ConstraintTag {
    id: i32,
    label: String,
}

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "constraint_documents")]
struct ConstraintDocument {
    id: i32,
    account_id: i32,
    title: String,
    filename: String,
}

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "constraint_owners")]
struct Owner {
    id: i32,
    name: String,
}

/// `belongs_to` without `foreign_key` reads `<related model>_id` from this
/// model (`owner_id`), not `<this model>_id`.
#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "constraint_pets")]
struct ConstraintPet {
    id: i32,
    owner_id: i32,
    name: String,
    #[orm(belongs_to = "Owner")]
    #[sqlx(default, skip)]
    owner: Option<Owner>,
}

async fn setup() {
    Orm::init_with_options("sqlite::memory:", 1, 5)
        .await
        .expect("initialize SQLite");
    let pool = Orm::pool().expect("pool");
    for statement in [
        "CREATE TABLE constraint_accounts (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
        "CREATE TABLE constraint_documents (id INTEGER PRIMARY KEY, account_id INTEGER NOT NULL, title TEXT NOT NULL, filename TEXT NOT NULL)",
        "INSERT INTO constraint_accounts (id, name) VALUES (1, 'ada'), (2, 'grace')",
        "INSERT INTO constraint_documents (id, account_id, title, filename) VALUES \
            (1, 1, 'report', 'a.pdf'), (2, 2, 'other', 'report.pdf'), (3, 1, 'misc', 'b.txt')",
        "CREATE TABLE constraint_tags (id INTEGER PRIMARY KEY, label TEXT NOT NULL)",
        "CREATE TABLE constraint_account_tags (account_id INTEGER NOT NULL, tag_id INTEGER NOT NULL)",
        "INSERT INTO constraint_tags (id, label) VALUES (1, 'vip')",
        "INSERT INTO constraint_account_tags (account_id, tag_id) VALUES (1, 1)",
        "CREATE TABLE constraint_owners (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
        "CREATE TABLE constraint_pets (id INTEGER PRIMARY KEY, owner_id INTEGER NOT NULL, name TEXT NOT NULL)",
        "INSERT INTO constraint_owners (id, name) VALUES (1, 'ada'), (2, 'grace')",
        "INSERT INTO constraint_pets (id, owner_id, name) VALUES (1, 2, 'rex'), (2, 1, 'tom')",
    ] {
        rullst_orm::_sqlx::query(statement)
            .execute(pool)
            .await
            .expect("schema/seed statement");
    }
}

fn ids(documents: &[ConstraintDocument]) -> Vec<i32> {
    documents.iter().map(|document| document.id).collect()
}

/// An `or_where` in the modifier must not reach another parent's rows.
async fn constrained_relations_keep_the_ownership_predicate() {
    let account = ConstraintAccount::find(1)
        .await
        .expect("find account")
        .expect("account exists");
    let search = Arc::new(|query: ConstraintDocumentQueryBuilder| {
        query
            .where_like("title", "%report%")
            .or_where_like("filename", "%report%")
    });
    let lazy = account
        .documents_constrained(search)
        .await
        .expect("lazy constrained relation");
    assert_eq!(ids(&lazy), vec![1], "another account's document leaked");

    let eager = ConstraintAccount::query()
        .where_eq("id", 1)
        .with_documents_constrained(|query| {
            query
                .where_like("title", "%report%")
                .or_where_like("filename", "%report%")
        })
        .get()
        .await
        .expect("eager constrained relation");
    assert_eq!(ids(eager[0].documents.as_deref().expect("loaded")), vec![1]);
}

/// An eager `belongs_to_many` load marks parents without related rows as
/// loaded (`Some(vec![])`), like `has_many`, also when no parent has any.
async fn eager_many_to_many_loads_empty_parents() {
    let labels = |account: &ConstraintAccount| {
        account
            .tags
            .as_ref()
            .map(|tags| tags.iter().map(|tag| tag.label.clone()).collect::<Vec<_>>())
    };
    let accounts = ConstraintAccount::query()
        .order_by("id")
        .with_tags()
        .get()
        .await
        .expect("eager belongs_to_many");
    assert_eq!(
        accounts.iter().map(labels).collect::<Vec<_>>(),
        vec![Some(vec!["vip".to_string()]), Some(Vec::new())]
    );
    let untagged = ConstraintAccount::query()
        .where_eq("id", 2)
        .with_tags()
        .get()
        .await
        .expect("eager belongs_to_many without pivot rows");
    assert_eq!(labels(&untagged[0]), Some(Vec::new()));
}

async fn belongs_to_defaults_to_the_related_model_key() {
    let pet = ConstraintPet::find(1)
        .await
        .expect("find pet")
        .expect("pet exists");
    let owner = pet.owner().await.expect("lazy belongs_to").expect("owner");
    assert_eq!(owner.name, "grace");
    let pets = ConstraintPet::query()
        .order_by("id")
        .with_owner()
        .get()
        .await
        .expect("eager belongs_to");
    let owners = pets
        .iter()
        .map(|pet| pet.owner.as_ref().map(|owner| owner.id))
        .collect::<Vec<_>>();
    assert_eq!(owners, vec![Some(2), Some(1)]);
}

#[tokio::test]
async fn relation_queries_stay_bound_to_their_parent() {
    setup().await;
    constrained_relations_keep_the_ownership_predicate().await;
    eager_many_to_many_loads_empty_parents().await;
    belongs_to_defaults_to_the_related_model_key().await;
}
