//! Filtered/tenant-scoped `delete_all()` and `cascade_soft_delete` must run on
//! every driver (MySQL/MariaDB reject PostgreSQL `$n` markers).

use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{FromRow, Orm, with_tenant};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "contract_bulk_items", tenant_column = "tenant_id")]
struct ContractBulkItem {
    id: i32,
    tenant_id: String,
    score: i32,
}

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "contract_bulk_posts")]
struct ContractBulkPost {
    id: i32,
    title: String,
    #[sqlx(default, skip)]
    #[orm(
        has_many = "ContractBulkComment",
        foreign_key = "post_id",
        cascade_soft_delete
    )]
    comments: Option<Vec<ContractBulkComment>>,
    deleted_at: Option<String>,
}

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "contract_bulk_comments")]
struct ContractBulkComment {
    id: i32,
    post_id: i32,
    body: String,
    deleted_at: Option<String>,
}

async fn insert_item(tenant: &str, score: i32) {
    with_tenant(tenant.to_string(), async move {
        let mut item = ContractBulkItem {
            id: 0,
            tenant_id: String::new(),
            score,
        };
        item.save().await.expect("insert tenant bulk item");
        assert_eq!(item.tenant_id, tenant);
    })
    .await;
}

pub(super) async fn exercise() {
    let driver = Orm::driver().expect("initialized driver");
    Schema::create("contract_bulk_items", |table: &mut Blueprint| {
        table.id();
        table.string("tenant_id").not_null();
        table.integer("score").not_null();
    })
    .await
    .expect("create bulk item contract table");
    for (tenant, score) in [("acme", 1), ("acme", 1), ("acme", 2), ("other", 1)] {
        insert_item(tenant, score).await;
    }

    let deleted = with_tenant("acme", async {
        ContractBulkItem::query()
            .where_eq("score", 1)
            .delete_all()
            .await
    })
    .await
    .unwrap_or_else(|error| panic!("{driver} filtered tenant delete_all: {error}"));
    assert_eq!(deleted, 2, "{driver}");
    let acme = with_tenant("acme", async { ContractBulkItem::query().get().await })
        .await
        .expect("remaining tenant rows");
    assert_eq!(
        acme.iter().map(|item| item.score).collect::<Vec<_>>(),
        vec![2]
    );
    assert!(acme.iter().all(|item| item.id > 0));
    let other = with_tenant("other", async { ContractBulkItem::query().count().await })
        .await
        .expect("other tenant rows");
    assert_eq!(other, 1, "{driver} must not delete another tenant's rows");

    // A mistyped tenant context fails closed instead of reaching `tenant_id = 5`,
    // which MySQL/MariaDB would compare numerically with '5' and '5-acme'.
    for tenant in ["5", "5-acme"] {
        insert_item(tenant, 1).await;
    }
    let mistyped_read = with_tenant(5_i32, async { ContractBulkItem::query().get().await }).await;
    let mistyped_delete = with_tenant(5_i32, async {
        ContractBulkItem::query().delete_all().await
    })
    .await;
    for (operation, failed_closed) in [
        (
            "read",
            matches!(&mistyped_read, Err(rullst_orm::Error::Validation(message)) if message.contains("tenant context type")),
        ),
        (
            "delete_all",
            matches!(&mistyped_delete, Err(rullst_orm::Error::Validation(message)) if message.contains("tenant context type")),
        ),
    ] {
        assert!(
            failed_closed,
            "{driver} mistyped tenant {operation} must fail closed"
        );
    }
    for tenant in ["5", "5-acme"] {
        let rows = with_tenant(tenant, async { ContractBulkItem::query().count().await })
            .await
            .expect("typed tenant count");
        assert_eq!(rows, 1, "{driver} {tenant}");
    }

    Schema::create("contract_bulk_posts", |table: &mut Blueprint| {
        table.id();
        table.string("title").not_null();
        table.soft_deletes();
    })
    .await
    .expect("create bulk post contract table");
    Schema::create("contract_bulk_comments", |table: &mut Blueprint| {
        table.id();
        table.integer("post_id").not_null();
        table.string("body").not_null();
        table.soft_deletes();
    })
    .await
    .expect("create bulk comment contract table");

    let mut posts = Vec::new();
    for title in ["cascade", "kept"] {
        let mut post = ContractBulkPost {
            id: 0,
            title: title.to_string(),
            comments: None,
            deleted_at: None,
        };
        post.save().await.expect("insert bulk post");
        posts.push(post);
    }
    for (post_index, body) in [(0, "first"), (0, "second"), (1, "kept")] {
        let mut comment = ContractBulkComment {
            id: 0,
            post_id: posts[post_index].id,
            body: body.to_string(),
            deleted_at: None,
        };
        comment.save().await.expect("insert bulk comment");
    }

    posts[0]
        .delete()
        .await
        .unwrap_or_else(|error| panic!("{driver} cascade_soft_delete: {error}"));
    assert!(
        ContractBulkPost::find(posts[0].id)
            .await
            .expect("read soft-deleted post")
            .is_none()
    );
    let visible = ContractBulkComment::query()
        .get()
        .await
        .expect("visible comments");
    assert_eq!(
        visible
            .iter()
            .map(|comment| (comment.post_id, comment.body.as_str()))
            .collect::<Vec<_>>(),
        vec![(posts[1].id, "kept")],
        "{driver} must soft-delete exactly the parent's children"
    );
    let trashed = ContractBulkComment::query()
        .only_trashed()
        .get()
        .await
        .expect("trashed comments");
    assert_eq!(trashed.len(), 2);
    assert!(trashed.iter().all(|comment| comment.deleted_at.is_some()));
    assert!(posts[1].title == "kept" && posts[1].comments.is_none());

    for table in [
        "contract_bulk_comments",
        "contract_bulk_posts",
        "contract_bulk_items",
    ] {
        Schema::drop_if_exists(table)
            .await
            .expect("drop bulk delete contract table");
    }
}
