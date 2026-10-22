//! A model with a `RowSnapshot` field writes only its changed columns, so two
//! handles that change different columns both persist. PostgreSQL and SQLite
//! return the stored row from the UPDATE; MySQL/MariaDB re-read it.

use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{Error, FromRow, Orm, RowSnapshot, with_tenant};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "contract_tracked_profiles")]
struct ContractTrackedProfile {
    id: i32,
    name: String,
    bio: String,
    xp: i32,
    #[sqlx(skip)]
    snapshot: RowSnapshot,
}

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "contract_tracked_cards", tenant_column = "tenant_id")]
struct ContractTrackedCard {
    id: i32,
    tenant_id: String,
    title: String,
    body: String,
    #[sqlx(skip)]
    snapshot: RowSnapshot,
}

async fn stored(id: i32) -> ContractTrackedProfile {
    ContractTrackedProfile::find(id)
        .await
        .expect("load profile")
        .expect("stored profile")
}

/// Adds `points` with one statement, outside the model.
async fn award(id: i32, points: i32) {
    let sql = if Orm::driver().expect("initialized driver") == "postgres" {
        "UPDATE contract_tracked_profiles SET xp = xp + $1 WHERE id = $2"
    } else {
        "UPDATE contract_tracked_profiles SET xp = xp + ? WHERE id = ?"
    };
    let updated = Orm::raw(sql)
        .bind(points)
        .bind(id)
        .execute()
        .await
        .expect("atomic increment");
    assert_eq!(updated, 1);
}

pub(super) async fn exercise() {
    let driver = Orm::driver().expect("initialized driver");
    Schema::create("contract_tracked_profiles", |table: &mut Blueprint| {
        table.id();
        table.string("name").not_null();
        table.string("bio").not_null();
        table.integer("xp").not_null();
    })
    .await
    .expect("create tracked profile contract table");
    Schema::create("contract_tracked_cards", |table: &mut Blueprint| {
        table.id();
        table.string("tenant_id").not_null();
        table.string("title").not_null();
        table.string("body").not_null();
    })
    .await
    .expect("create tracked card contract table");

    let mut profile = ContractTrackedProfile {
        id: 0,
        name: "Ada".to_string(),
        bio: String::new(),
        xp: 0,
        snapshot: RowSnapshot::default(),
    };
    profile.save().await.expect("insert profile");
    assert!(!profile.is_dirty(), "{driver} an inserted record is clean");

    // Two handles change different columns; both changes persist.
    let mut first = stored(profile.id).await;
    let mut second = stored(profile.id).await;
    first.name = "Ada Lovelace".to_string();
    second.bio = "Analyst".to_string();
    first.save().await.expect("first save");
    second.save().await.expect("second save");
    let both = stored(profile.id).await;
    assert_eq!(both.name, "Ada Lovelace", "{driver} first change kept");
    assert_eq!(both.bio, "Analyst", "{driver} second change kept");
    assert_eq!(
        second.name, "Ada Lovelace",
        "{driver} the unwritten column is refreshed"
    );

    // A concurrent counter increment survives a profile edit.
    let mut editor = stored(profile.id).await;
    award(profile.id, 5).await;
    editor.bio = "Mathematician".to_string();
    assert_eq!(editor.dirty_columns(), ["bio"], "{driver}");
    editor.save().await.expect("edit over increment");
    assert_eq!(stored(profile.id).await.xp, 5, "{driver} increment kept");
    assert_eq!(editor.xp, 5, "{driver} handle refreshed");
    // Unchanged: nothing to write.
    editor.save().await.expect("unchanged save");

    // mark_all_dirty() overwrites like the full-row save.
    award(profile.id, 5).await;
    editor.mark_all_dirty();
    editor.save().await.expect("full-row save");
    assert_eq!(stored(profile.id).await.xp, 5, "{driver} full row wins");

    // A changed save of a deleted row reports RecordNotFound.
    let mut vanished = stored(profile.id).await;
    stored(profile.id)
        .await
        .delete()
        .await
        .expect("delete profile");
    vanished.name = "Gone".to_string();
    let saved = vanished.save().await;
    assert!(
        matches!(saved, Err(Error::RecordNotFound)),
        "{driver} save() of a deleted row: {saved:?}"
    );

    // Tenant-scoped: the UPDATE keeps the tenant predicate.
    let card_id = with_tenant("acme", async {
        let mut card = ContractTrackedCard {
            id: 0,
            tenant_id: String::new(),
            title: "card".to_string(),
            body: "body".to_string(),
            snapshot: RowSnapshot::default(),
        };
        card.save().await.expect("insert card");
        let mut first = ContractTrackedCard::find(card.id)
            .await
            .expect("load card")
            .expect("stored card");
        let mut second = first.clone();
        first.title = "title".to_string();
        second.body = "edited body".to_string();
        first.save().await.expect("first card save");
        second.save().await.expect("second card save");
        card.id
    })
    .await;
    let (title, body) = with_tenant("acme", async {
        let card = ContractTrackedCard::find(card_id)
            .await
            .expect("reload card")
            .expect("stored card");
        (card.title, card.body)
    })
    .await;
    assert_eq!(title, "title", "{driver}");
    assert_eq!(body, "edited body", "{driver}");
    let mut foreign = with_tenant("acme", async {
        ContractTrackedCard::find(card_id)
            .await
            .expect("load card")
            .expect("stored card")
    })
    .await;
    foreign.title = "hijack".to_string();
    let hijack = with_tenant("globex", foreign.save()).await;
    assert!(
        matches!(hijack, Err(Error::Validation(_))),
        "{driver} another tenant: {hijack:?}"
    );

    Schema::drop_if_exists("contract_tracked_cards")
        .await
        .expect("drop tracked card contract table");
    Schema::drop_if_exists("contract_tracked_profiles")
        .await
        .expect("drop tracked profile contract table");
}
