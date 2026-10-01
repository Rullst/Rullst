//! The mandatory tenant and soft-delete predicates and the `chunk_by_id`
//! keyset are qualified with the model's table, so a join with a table that
//! has the same columns (a tenant-scoped, soft-deletable pivot, say) cannot
//! make them ambiguous.

use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{FromRow, Orm, with_tenant};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "contract_scoped_posts", tenant_column = "organization_id")]
struct ContractScopedPost {
    id: i32,
    organization_id: String,
    title: String,
    #[sqlx(default, skip)]
    #[orm(
        belongs_to_many = "ContractScopedTag",
        pivot_table = "contract_scoped_post_tags",
        foreign_key = "post_id",
        related_key = "tag_id"
    )]
    tags: Option<Vec<ContractScopedTag>>,
    deleted_at: Option<String>,
}

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "contract_scoped_tags", tenant_column = "organization_id")]
struct ContractScopedTag {
    id: i32,
    organization_id: String,
    label: String,
    deleted_at: Option<String>,
}

const PIVOT: &str = "contract_scoped_post_tags";

fn labels(tags: &[ContractScopedTag]) -> Vec<&str> {
    let mut labels = tags
        .iter()
        .map(|tag| tag.label.as_str())
        .collect::<Vec<_>>();
    labels.sort_unstable();
    labels
}

async fn create_tables() {
    Schema::create("contract_scoped_posts", |table: &mut Blueprint| {
        table.id();
        table.string("organization_id").not_null();
        table.string("title").not_null();
        table.soft_deletes();
    })
    .await
    .expect("create scoped post table");
    Schema::create("contract_scoped_tags", |table: &mut Blueprint| {
        table.id();
        table.string("organization_id").not_null();
        table.string("label").not_null();
        table.soft_deletes();
    })
    .await
    .expect("create scoped tag table");
    // The pivot carries the same tenant and soft-delete columns.
    Schema::create(PIVOT, |table: &mut Blueprint| {
        table.id();
        table.integer("post_id").not_null();
        table.integer("tag_id").not_null();
        table.string("organization_id").not_null();
        table.soft_deletes();
    })
    .await
    .expect("create scoped pivot table");
}

/// Seeds two posts of `acme`, three tags (one trashed) and the pivot rows.
async fn seed() -> (ContractScopedPost, ContractScopedPost) {
    with_tenant("acme", async {
        let mut posts = Vec::new();
        for title in ["tagged", "other"] {
            let mut post = ContractScopedPost {
                id: 0,
                organization_id: String::new(),
                title: title.to_string(),
                tags: None,
                deleted_at: None,
            };
            post.save().await.expect("insert scoped post");
            posts.push(post);
        }
        let mut tags = Vec::new();
        for label in ["rust", "sql", "trashed"] {
            let mut tag = ContractScopedTag {
                id: 0,
                organization_id: String::new(),
                label: label.to_string(),
                deleted_at: None,
            };
            tag.save().await.expect("insert scoped tag");
            tags.push(tag);
        }
        tags[2].delete().await.expect("trash a tag");
        let pool = Orm::pool().expect("pool");
        for (post, tag) in [(0, 0), (0, 1), (0, 2), (1, 1)] {
            let insert = format!(
                "INSERT INTO {PIVOT} (post_id, tag_id, organization_id) VALUES ({}, {}, 'acme')",
                posts[post].id, tags[tag].id
            );
            rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(insert.as_str()))
                .execute(pool)
                .await
                .expect("insert pivot row");
        }
        let other = posts.pop().expect("second post");
        let tagged = posts.pop().expect("first post");
        (tagged, other)
    })
    .await
}

pub(super) async fn exercise() {
    let driver = Orm::driver().expect("initialized driver");
    create_tables().await;
    let (tagged, other) = seed().await;

    with_tenant("acme", async {
        // The lazy loader joins the pivot onto the scoped tag query.
        let lazy = tagged
            .tags()
            .await
            .unwrap_or_else(|error| panic!("{driver} lazy belongs_to_many: {error}"));
        assert_eq!(labels(&lazy), ["rust", "sql"], "{driver}");

        let eager = ContractScopedPost::query()
            .order_by("id")
            .with_tags()
            .get()
            .await
            .unwrap_or_else(|error| panic!("{driver} eager belongs_to_many: {error}"));
        let eager_labels = eager
            .iter()
            .map(|post| labels(post.tags.as_deref().expect("loaded")))
            .collect::<Vec<_>>();
        assert_eq!(eager_labels, [vec!["rust", "sql"], vec!["sql"]], "{driver}");

        // A join with the pivot keeps both mandatory scopes unambiguous.
        let rust = lazy
            .iter()
            .find(|tag| tag.label == "rust")
            .expect("rust tag");
        let joined = ContractScopedPost::query()
            .select_raw("contract_scoped_posts.*")
            .join(
                PIVOT,
                "contract_scoped_post_tags.post_id",
                "=",
                "contract_scoped_posts.id",
            )
            .where_eq("contract_scoped_post_tags.tag_id", rust.id);
        let count = joined
            .count()
            .await
            .unwrap_or_else(|error| panic!("{driver} joined count: {error}"));
        assert_eq!(count, 1, "{driver}");

        let mut seen = Vec::new();
        ContractScopedPost::query()
            .select_raw("contract_scoped_posts.*")
            .join(
                PIVOT,
                "contract_scoped_post_tags.post_id",
                "=",
                "contract_scoped_posts.id",
            )
            .chunk_by_id(1, |posts| {
                seen.extend(posts.iter().map(|post| post.id));
                async { Ok(()) }
            })
            .await
            .unwrap_or_else(|error| panic!("{driver} joined chunk_by_id: {error}"));
        assert_eq!(seen, [tagged.id, other.id], "{driver}");
    })
    .await;

    for table in [PIVOT, "contract_scoped_tags", "contract_scoped_posts"] {
        Schema::drop_if_exists(table)
            .await
            .expect("drop qualified scope contract table");
    }
}
