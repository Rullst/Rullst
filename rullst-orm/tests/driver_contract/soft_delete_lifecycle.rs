//! `restore()` and `force_delete()` must execute on every driver, including
//! the tenant predicate (PostgreSQL rejects raw `?` markers).

use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{Error, FromRow, Orm, with_tenant};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "contract_archived_notes", tenant_column = "tenant_id")]
struct ContractArchivedNote {
    id: i32,
    tenant_id: String,
    title: String,
    deleted_at: Option<String>,
}

async fn trashed(id: i32) -> Option<ContractArchivedNote> {
    ContractArchivedNote::query()
        .with_trashed()
        .where_id(id)
        .first()
        .await
        .expect("read note including trashed rows")
}

pub(super) async fn exercise() {
    let driver = Orm::driver().expect("initialized driver");
    Schema::create("contract_archived_notes", |table: &mut Blueprint| {
        table.id();
        table.string("tenant_id").not_null();
        table.string("title").not_null();
        table.soft_deletes();
    })
    .await
    .expect("create soft-delete contract table");

    with_tenant("acme", async {
        let mut note = ContractArchivedNote {
            id: 0,
            tenant_id: String::new(),
            title: "archived".to_string(),
            deleted_at: None,
        };
        note.save().await.expect("insert note");
        note.delete().await.expect("soft delete note");
        assert!(ContractArchivedNote::find(note.id).await.unwrap().is_none());
        let deleted = trashed(note.id).await.expect("soft-deleted row remains");
        assert!(deleted.deleted_at.is_some());

        deleted
            .restore()
            .await
            .unwrap_or_else(|error| panic!("{driver} restore(): {error}"));
        let restored = ContractArchivedNote::find(note.id)
            .await
            .unwrap()
            .expect("restored row is visible");
        assert_eq!(restored.title, "archived");
        assert!(restored.deleted_at.is_none());

        restored
            .force_delete()
            .await
            .unwrap_or_else(|error| panic!("{driver} force_delete(): {error}"));
        assert!(
            trashed(note.id).await.is_none(),
            "{driver} row must be gone"
        );
    })
    .await;

    let foreign = with_tenant("acme", async {
        let mut note = ContractArchivedNote {
            id: 0,
            tenant_id: String::new(),
            title: "foreign".to_string(),
            deleted_at: None,
        };
        note.save().await.expect("insert foreign note");
        note
    })
    .await;
    let cross_tenant = with_tenant("other", async { foreign.force_delete().await }).await;
    assert!(matches!(cross_tenant, Err(Error::Validation(_))));
    let still_there = with_tenant("acme", trashed(foreign.id)).await;
    assert!(
        still_there.is_some(),
        "{driver} cross-tenant delete must not run"
    );

    Schema::drop_if_exists("contract_archived_notes")
        .await
        .expect("drop soft-delete contract table");
}
