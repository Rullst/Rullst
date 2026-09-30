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
}

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "constraint_documents")]
struct ConstraintDocument {
    id: i32,
    account_id: i32,
    title: String,
    filename: String,
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

#[tokio::test]
async fn relation_queries_stay_bound_to_their_parent() {
    setup().await;
    constrained_relations_keep_the_ownership_predicate().await;
}
