//! `restore()` and `force_delete()` statements per driver. PostgreSQL must
//! receive `$n` markers; MySQL/MariaDB and SQLite keep `?`. The live matrices
//! execute both methods through `driver_contract`.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::FromRow;

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "archived_notes", tenant_column = "tenant_id")]
struct ArchivedNote {
    id: i32,
    tenant_id: String,
    deleted_at: Option<String>,
}

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(
    table = "flagged_notes",
    soft_delete(column = "archived", value = "0", delval = "1")
)]
struct FlaggedNote {
    id: i32,
    archived: i32,
}

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
// v12's derive needs at least one column besides `id`.
#[orm(table = "plain_notes")]
struct PlainNote {
    id: i32,
    body: String,
}

const NON_POSTGRES: [&str; 2] = ["mysql", "sqlite"];

#[test]
fn restore_statements_are_numbered_only_for_postgres() {
    assert_eq!(
        ArchivedNote::__rullst_restore_sql("postgres"),
        "UPDATE archived_notes SET deleted_at = NULL WHERE id = $1 AND tenant_id = $2"
    );
    assert_eq!(
        FlaggedNote::__rullst_restore_sql("postgres"),
        "UPDATE flagged_notes SET archived = 0 WHERE id = $1"
    );
    for driver in NON_POSTGRES {
        assert_eq!(
            ArchivedNote::__rullst_restore_sql(driver),
            "UPDATE archived_notes SET deleted_at = NULL WHERE id = ? AND tenant_id = ?",
            "{driver}"
        );
        assert_eq!(
            FlaggedNote::__rullst_restore_sql(driver),
            "UPDATE flagged_notes SET archived = 0 WHERE id = ?",
            "{driver}"
        );
    }
}

#[test]
fn force_delete_statements_are_numbered_only_for_postgres() {
    assert_eq!(
        ArchivedNote::__rullst_force_delete_sql("postgres"),
        "DELETE FROM archived_notes WHERE id = $1 AND tenant_id = $2"
    );
    assert_eq!(
        PlainNote::__rullst_force_delete_sql("postgres"),
        "DELETE FROM plain_notes WHERE id = $1"
    );
    for driver in NON_POSTGRES {
        assert_eq!(
            ArchivedNote::__rullst_force_delete_sql(driver),
            "DELETE FROM archived_notes WHERE id = ? AND tenant_id = ?",
            "{driver}"
        );
        assert_eq!(
            PlainNote::__rullst_force_delete_sql(driver),
            "DELETE FROM plain_notes WHERE id = ?",
            "{driver}"
        );
    }
}

#[test]
fn fixture_models_keep_their_fields() {
    let note = ArchivedNote {
        id: 1,
        tenant_id: "acme".to_string(),
        deleted_at: None,
    };
    let flagged = FlaggedNote { id: 2, archived: 0 };
    let plain = PlainNote {
        id: 3,
        body: String::new(),
    };
    assert_eq!(
        (note.id, note.tenant_id.as_str(), note.deleted_at),
        (1, "acme", None)
    );
    assert_eq!((flagged.id, flagged.archived, plain.id), (2, 0, 3));
}
