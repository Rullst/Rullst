//! By-ID mutations of a row that no longer exists must not report success
//! (MySQL/MariaDB count matched rows only with `CLIENT_FOUND_ROWS`).

use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{Error, FromRow, Orm, with_tenant};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "contract_vanishing_notes")]
struct ContractVanishingNote {
    id: i32,
    title: String,
}

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "contract_vanishing_cards", tenant_column = "tenant_id")]
struct ContractVanishingCard {
    id: i32,
    tenant_id: String,
    title: String,
    deleted_at: Option<String>,
}

async fn note_count() -> usize {
    ContractVanishingNote::all()
        .await
        .expect("list contract notes")
        .len()
}

pub(super) async fn exercise() {
    let driver = Orm::driver().expect("initialized driver");
    Schema::create("contract_vanishing_notes", |table: &mut Blueprint| {
        table.id();
        table.string("title").not_null();
    })
    .await
    .expect("create missing-row contract table");
    Schema::create("contract_vanishing_cards", |table: &mut Blueprint| {
        table.id();
        table.string("tenant_id").not_null();
        table.string("title").not_null();
        table.soft_deletes();
    })
    .await
    .expect("create tenant missing-row contract table");

    let mut note = ContractVanishingNote {
        id: 0,
        title: "draft".to_string(),
    };
    note.save().await.expect("insert note");
    // Saving the unchanged row still matches it on every driver.
    note.save().await.expect("save unchanged note");
    let stale = note.clone();
    note.delete().await.expect("delete note");

    let mut edited = stale.clone();
    edited.title = "edited after delete".to_string();
    let saved = edited.save().await;
    assert!(
        matches!(saved, Err(Error::RecordNotFound)),
        "{driver} save() of a deleted row: {saved:?}"
    );
    assert_eq!(edited.id, stale.id, "{driver} failed update keeps the id");
    let deleted_again = stale.delete().await;
    assert!(
        matches!(deleted_again, Err(Error::RecordNotFound)),
        "{driver} delete() of a deleted row: {deleted_again:?}"
    );
    assert_eq!(note_count().await, 0, "{driver} nothing was recreated");

    // A tenant model restores a missing row as the documented no-op.
    with_tenant("acme", async {
        let mut card = ContractVanishingCard {
            id: 0,
            tenant_id: String::new(),
            title: "card".to_string(),
            deleted_at: None,
        };
        card.save().await.expect("insert tenant card");
        // restore() of a live row and a second delete() change nothing.
        card.restore()
            .await
            .unwrap_or_else(|error| panic!("{driver} restore() of a live row: {error}"));
        card.delete().await.expect("soft delete tenant card");
        let again = card.delete().await;
        assert!(
            matches!(again, Err(Error::Validation(_))),
            "{driver} delete() of a trashed row: {again:?}"
        );
        let handle = card.clone();
        card.force_delete().await.expect("force delete tenant card");
        handle
            .restore()
            .await
            .unwrap_or_else(|error| panic!("{driver} restore() of a missing row: {error}"));
    })
    .await;

    Schema::drop_if_exists("contract_vanishing_cards")
        .await
        .expect("drop tenant missing-row contract table");
    Schema::drop_if_exists("contract_vanishing_notes")
        .await
        .expect("drop missing-row contract table");
}
