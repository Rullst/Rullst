//! PostgreSQL bind-marker numbering for typed subqueries.
//!
//! A generated builder renders `$n` markers from `to_sql()` when the active
//! driver is PostgreSQL. `PostgresRendered` reproduces that rendering without
//! a live server, so these assertions pin the final statement text and the
//! ordered binding vector that PostgreSQL would receive.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::schema::SubqueryBuilder;
use rullst_orm::{Error, RullstValue, replace_placeholders, with_tenant};

#[derive(Clone, Debug, rullst_orm::Orm, rullst_orm::FromRow)]
#[orm(table = "subquery_posts", tenant_column = "tenant_id")]
struct SubqueryPost {
    id: i32,
    tenant_id: String,
    author_id: i32,
    title: String,
}

#[derive(Clone, Debug, rullst_orm::Orm, rullst_orm::FromRow)]
#[orm(table = "subquery_comments", tenant_column = "tenant_id")]
struct SubqueryComment {
    id: i32,
    tenant_id: String,
    post_id: i32,
    status: String,
}

/// Renders a typed builder exactly as `to_sql()` does on PostgreSQL.
struct PostgresRendered<B>(B);

impl<B: SubqueryBuilder> SubqueryBuilder for PostgresRendered<B> {
    fn to_sql(&self) -> String {
        replace_placeholders(&self.0.to_sql())
    }

    fn bindings(&self) -> &Vec<RullstValue> {
        self.0.bindings()
    }

    fn ordered_bindings(&self) -> Vec<RullstValue> {
        self.0.ordered_bindings()
    }

    fn validation_error(&self) -> Option<Error> {
        self.0.validation_error()
    }
}

/// `RullstValue` has no `PartialEq`; its `Debug` form identifies type and value.
fn described(values: &[RullstValue]) -> Vec<String> {
    values.iter().map(|value| format!("{value:?}")).collect()
}

fn texts(values: &[&str]) -> Vec<String> {
    values
        .iter()
        .map(|value| format!("{:?}", RullstValue::String((*value).to_string())))
        .collect()
}

/// Renders the outer statement as PostgreSQL would receive it.
fn postgres_statement(query: &impl SubqueryBuilder) -> (String, Vec<RullstValue>) {
    (
        replace_placeholders(&query.to_sql()),
        query.ordered_bindings(),
    )
}

/// Every `$n` must appear exactly once, in textual order, matching one binding.
fn assert_sequential_markers(sql: &str, bindings: &[RullstValue]) {
    let markers: Vec<usize> = sql
        .split('$')
        .skip(1)
        .map(|tail| {
            tail.chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
                .parse()
                .expect("numbered marker")
        })
        .collect();
    let expected: Vec<usize> = (1..=bindings.len()).collect();
    assert_eq!(
        markers, expected,
        "markers must follow binding order: {sql}"
    );
}

#[tokio::test]
async fn exists_subquery_keeps_the_mandatory_tenant_binding_first() {
    let (sql, bindings) = with_tenant("acme", async {
        let query = SubqueryPost::query().where_exists(PostgresRendered(
            SubqueryComment::query()
                .where_column("subquery_comments.post_id", "subquery_posts.id")
                .where_eq("status", "user-input"),
        ));
        assert!(query.errors.is_empty(), "{:?}", query.errors);
        postgres_statement(&query)
    })
    .await;

    assert_eq!(
        sql,
        "SELECT * FROM subquery_posts WHERE (tenant_id = $1) AND \
         (EXISTS (SELECT * FROM subquery_comments WHERE (tenant_id = $2) AND \
         ((subquery_comments.post_id = subquery_posts.id) AND (status = $3)) LIMIT 1000)) \
         LIMIT 1000"
    );
    assert_eq!(described(&bindings), texts(&["acme", "acme", "user-input"]));
}

#[tokio::test]
async fn nested_ctes_joins_and_exists_number_every_marker_once() {
    let (sql, bindings) = with_tenant("acme", async {
        let query = SubqueryPost::query()
            .where_eq("title", "hello")
            .with_cte(
                "published",
                PostgresRendered(SubqueryComment::query().where_eq("status", "published")),
            )
            .join_constrained("subquery_authors", |join| {
                join.on("subquery_authors.id", "=", "subquery_posts.author_id")
                    .on_eq("subquery_authors.region", "eu")
            })
            .where_exists(PostgresRendered(
                SubqueryComment::query()
                    .where_column("subquery_comments.post_id", "subquery_posts.id")
                    .where_exists(PostgresRendered(
                        SubqueryComment::query().where_eq("status", "nested"),
                    ))
                    .where_eq("status", "user-input"),
            ))
            .with_cte(
                "flagged",
                PostgresRendered(
                    SubqueryComment::query()
                        .with_cte(
                            "deepest",
                            PostgresRendered(SubqueryComment::query().where_eq("status", "deep")),
                        )
                        .where_eq("status", "flagged"),
                ),
            )
            .or_where_exists(PostgresRendered(
                SubqueryComment::query().where_eq("status", "either"),
            ));
        assert!(query.errors.is_empty(), "{:?}", query.errors);
        postgres_statement(&query)
    })
    .await;

    assert_eq!(
        sql,
        "WITH published AS (SELECT * FROM subquery_comments WHERE (tenant_id = $1) AND \
         (status = $2) LIMIT 1000), flagged AS (WITH deepest AS (SELECT * FROM \
         subquery_comments WHERE (tenant_id = $3) AND (status = $4) LIMIT 1000) SELECT * \
         FROM subquery_comments WHERE (tenant_id = $5) AND (status = $6) LIMIT 1000) \
         SELECT * FROM subquery_posts INNER JOIN subquery_authors ON subquery_authors.id = \
         subquery_posts.author_id AND subquery_authors.region = $7 WHERE (tenant_id = $8) AND \
         ((title = $9) AND (EXISTS (SELECT * FROM subquery_comments WHERE (tenant_id = $10) \
         AND ((subquery_comments.post_id = subquery_posts.id) AND (EXISTS (SELECT * FROM \
         subquery_comments WHERE (tenant_id = $11) AND (status = $12) LIMIT 1000)) AND \
         (status = $13)) LIMIT 1000)) OR (EXISTS (SELECT * FROM subquery_comments WHERE \
         (tenant_id = $14) AND (status = $15) LIMIT 1000))) LIMIT 1000"
    );
    assert_eq!(
        described(&bindings),
        texts(&[
            "acme",
            "published",
            "acme",
            "deep",
            "acme",
            "flagged",
            "eu",
            "acme",
            "hello",
            "acme",
            "acme",
            "nested",
            "user-input",
            "acme",
            "either",
        ])
    );
    assert_sequential_markers(&sql, &bindings);
}

#[test]
fn inconsistent_subquery_numbering_fails_closed() {
    struct Broken;
    impl SubqueryBuilder for Broken {
        fn to_sql(&self) -> String {
            "SELECT 1 WHERE a = $2".to_string()
        }
        fn bindings(&self) -> &Vec<RullstValue> {
            static EMPTY: Vec<RullstValue> = Vec::new();
            &EMPTY
        }
    }

    let query = SubqueryPost::unscoped().where_exists(Broken);
    assert!(matches!(query.errors.first(), Some(Error::Validation(_))));
    assert!(query.to_sql().contains("(1 = 0)"));
    assert!(
        !SubqueryPost::unscoped()
            .with_cte("broken", Broken)
            .errors
            .is_empty()
    );
}

#[cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#[tokio::test]
async fn tenant_scoped_subqueries_execute_with_the_expected_rows() {
    use rullst_orm::Orm;

    Orm::init_with_options(
        "sqlite:file:subquery_placeholder_test.db?mode=memory&cache=shared",
        2,
        30,
    )
    .await
    .expect("initialize subquery database");
    let pool = Orm::pool().expect("subquery pool");
    for statement in [
        "CREATE TABLE subquery_posts (id INTEGER PRIMARY KEY, tenant_id TEXT NOT NULL, author_id INTEGER NOT NULL, title TEXT NOT NULL)",
        "CREATE TABLE subquery_comments (id INTEGER PRIMARY KEY, tenant_id TEXT NOT NULL, post_id INTEGER NOT NULL, status TEXT NOT NULL)",
        "CREATE TABLE subquery_authors (id INTEGER PRIMARY KEY, region TEXT NOT NULL)",
        "INSERT INTO subquery_posts VALUES (1, 'acme', 1, 'hello'), (2, 'acme', 2, 'hello'), (3, 'other', 1, 'hello'), (4, 'acme', 1, 'hello')",
        "INSERT INTO subquery_comments VALUES (1, 'acme', 1, 'open'), (2, 'acme', 2, 'open'), (3, 'other', 3, 'open'), (4, 'acme', 4, 'published'), (5, 'other', 4, 'open')",
        "INSERT INTO subquery_authors VALUES (1, 'eu'), (2, 'us')",
    ] {
        rullst_orm::_sqlx::query(statement)
            .execute(pool)
            .await
            .expect("seed subquery fixture");
    }

    let open_comment = || {
        PostgresRendered(
            SubqueryComment::query()
                .where_column("subquery_comments.post_id", "subquery_posts.id")
                .where_eq("status", "open"),
        )
    };
    let (ids, count, deleted, remaining) = with_tenant("acme", async {
        let query = SubqueryPost::query()
            .with_cte(
                "published_comments",
                PostgresRendered(SubqueryComment::query().where_eq("status", "published")),
            )
            .join_constrained("subquery_authors", |join| {
                join.on("subquery_authors.id", "=", "subquery_posts.author_id")
                    .on_eq("subquery_authors.region", "eu")
            })
            .where_exists(open_comment());
        let ids = query
            .clone()
            .order_by("subquery_posts.id")
            .pluck_i32("subquery_posts.id")
            .await
            .expect("tenant-scoped EXISTS with CTE and JOIN");
        let count = query.count().await.expect("count with ordered bindings");
        let deleted = SubqueryPost::query()
            .where_exists(open_comment())
            .delete_all()
            .await
            .expect("delete_all with an embedded subquery");
        let remaining = SubqueryPost::query()
            .order_by("id")
            .pluck_i32("id")
            .await
            .expect("remaining tenant rows");
        (ids, count, deleted, remaining)
    })
    .await;

    // Post 2 has an open comment but a non-EU author; post 4's open comment
    // belongs to another tenant; post 3 is outside the active tenant.
    assert_eq!(ids, vec![1]);
    assert_eq!(count, 1);
    assert_eq!(deleted, 2);
    assert_eq!(remaining, vec![4]);
}
