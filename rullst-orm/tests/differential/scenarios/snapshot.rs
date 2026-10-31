//! Dirty tracking (`RowSnapshot`): partial `save()`, `update_partial()`,
//! `mark_all_dirty()` and database-managed timestamps refreshed after an
//! update.

use rullst_orm::chrono::{DateTime, Utc};
use rullst_orm::{Error, Orm, RowSnapshot};
use serde_json::{Value, json};

use super::create_table;
use crate::transcript::Scenario;

#[derive(Debug, Clone, Orm)]
#[orm(table = "diff_profiles")]
struct Profile {
    id: i32,
    name: String,
    bio: String,
    visits: i64,
    #[orm(db_default)]
    created_at: Option<DateTime<Utc>>,
    #[orm(db_default)]
    updated_at: Option<DateTime<Utc>>,
    #[sqlx(skip)]
    snapshot: RowSnapshot,
}

impl Profile {
    fn columns(&self) -> Value {
        json!([self.name, self.bio, self.visits])
    }
}

async fn load(id: i32) -> Result<Profile, Error> {
    Profile::find(id).await?.ok_or(Error::RecordNotFound)
}

async fn stored(id: i32) -> Result<Value, Error> {
    load(id).await.map(|profile| profile.columns())
}

pub async fn run(s: &mut Scenario<'_>) {
    let created = create_table(s, "diff_profiles", |table| {
        table.id();
        table.string("name").not_null();
        table.string("bio").not_null();
        table.big_integer("visits").not_null();
        table.timestamps();
    })
    .await;
    if !created {
        return;
    }
    let mut profile = Profile {
        id: 0,
        name: "Ada".to_string(),
        bio: "first".to_string(),
        visits: 1,
        created_at: None,
        updated_at: None,
        snapshot: RowSnapshot::default(),
    };
    let inserted = profile.save().await;
    let ok = inserted.is_ok();
    s.outcome("insert", inserted);
    if !ok {
        return;
    }
    let id = profile.id;
    s.record(
        "insert_keeps_db_default_fields_unset",
        profile.created_at.is_none(),
    );

    // Two handles change different columns; both changes survive.
    let (mut renamer, mut writer) = match (load(id).await, load(id).await) {
        (Ok(renamer), Ok(writer)) => (renamer, writer),
        (Err(error), _) | (_, Err(error)) => {
            return s.outcome("load_two_handles", Err::<(), _>(error));
        }
    };
    s.record("loaded_is_clean", !renamer.is_dirty());
    renamer.name = "Ada L.".to_string();
    writer.bio = "second".to_string();
    s.record(
        "dirty_columns",
        [renamer.dirty_columns(), writer.dirty_columns()],
    );
    s.outcome("save_renamer", renamer.save().await);
    s.outcome("save_writer", writer.save().await);
    s.outcome("merged_row", stored(id).await);
    s.record(
        "update_refreshes_db_default_fields",
        [renamer.created_at.is_some(), renamer.updated_at.is_some()],
    );
    s.record("saved_handle_is_clean", !renamer.is_dirty());

    // An unchanged handle saves nothing; `mark_all_dirty` writes every column.
    let mut stale = writer.clone();
    s.outcome("save_unchanged", stale.save().await);
    stale.mark_all_dirty();
    s.record("mark_all_dirty_columns", stale.dirty_columns());
    s.outcome("save_all_columns", stale.save().await);
    s.outcome("row_after_full_save", stored(id).await);

    let mut partial = renamer.clone();
    s.outcome(
        "update_partial",
        partial.update_partial().visits(42).save().await,
    );
    s.outcome("row_after_update_partial", stored(id).await);
    s.record("update_partial_handle", partial.columns());

    let mut vanished = match load(id).await {
        Ok(profile) => profile,
        Err(error) => return s.outcome("load_before_delete", Err::<(), _>(error)),
    };
    s.outcome("delete", partial.delete().await);
    vanished.bio = "too late".to_string();
    s.outcome("save_deleted_row", vanished.save().await);
    s.outcome(
        "update_partial_deleted_row",
        vanished.update_partial().visits(7).save().await,
    );
}
