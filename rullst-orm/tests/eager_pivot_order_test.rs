//! An eager `belongs_to_many` load keeps the related query's order (for
//! example a `with_<relation>_constrained` `order_by`), like the lazy
//! constrained loader, instead of the unordered pivot rows' order.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::sync::Arc;

use rullst_orm::{FromRow, Orm};

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "ordered_articles")]
struct OrderedArticle {
    id: i32,
    title: String,
    #[orm(
        belongs_to_many = "OrderedLabel",
        pivot_table = "ordered_article_labels",
        foreign_key = "article_id",
        related_key = "label_id"
    )]
    #[sqlx(default, skip)]
    labels: Option<Vec<OrderedLabel>>,
}

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "ordered_labels")]
struct OrderedLabel {
    id: i32,
    name: String,
}

fn names(article: &OrderedArticle) -> Vec<&str> {
    article
        .labels
        .as_deref()
        .expect("labels loaded")
        .iter()
        .map(|label| label.name.as_str())
        .collect()
}

async fn eager(descending: bool) -> Vec<OrderedArticle> {
    OrderedArticle::query()
        .order_by("id")
        .with_labels_constrained(move |query| {
            if descending {
                query.order_by_desc("name")
            } else {
                query.order_by("name")
            }
        })
        .get()
        .await
        .expect("eager belongs_to_many")
}

#[tokio::test]
async fn eager_many_to_many_follows_the_related_query_order() {
    Orm::init_with_options("sqlite::memory:", 1, 5)
        .await
        .expect("initialize SQLite");
    let pool = Orm::pool().expect("pool");
    for statement in [
        "CREATE TABLE ordered_articles (id INTEGER PRIMARY KEY, title TEXT NOT NULL)",
        "CREATE TABLE ordered_labels (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
        "CREATE TABLE ordered_article_labels (article_id INTEGER NOT NULL, label_id INTEGER NOT NULL)",
        "INSERT INTO ordered_articles (id, title) VALUES (1, 'first'), (2, 'second')",
        "INSERT INTO ordered_labels (id, name) VALUES (1, 'alpha'), (2, 'mid'), (3, 'zeta')",
        // Pivot rows deliberately follow neither the label IDs nor their names.
        "INSERT INTO ordered_article_labels (article_id, label_id) VALUES (1, 3), (2, 3), (1, 1), (1, 2), (2, 2)",
    ] {
        rullst_orm::_sqlx::query(statement)
            .execute(pool)
            .await
            .expect("schema/seed statement");
    }

    let ascending = eager(false).await;
    assert_eq!(names(&ascending[0]), ["alpha", "mid", "zeta"]);
    assert_eq!(names(&ascending[1]), ["mid", "zeta"]);
    let descending = eager(true).await;
    assert_eq!(names(&descending[0]), ["zeta", "mid", "alpha"]);
    assert_eq!(names(&descending[1]), ["zeta", "mid"]);

    // The lazy constrained loader returns the same order.
    let lazy = ascending[0]
        .labels_constrained(Arc::new(|query: OrderedLabelQueryBuilder| {
            query.order_by("name")
        }))
        .await
        .expect("lazy constrained belongs_to_many");
    let lazy_names = lazy
        .iter()
        .map(|label| label.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(lazy_names, names(&ascending[0]));
}
