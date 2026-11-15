//! 13.0.0-alpha.6: a model without an explicit save mode no longer compiles.

use super::*;

fn save_mode_lines(source: &str) -> Vec<usize> {
    scan_with(source, "models.rs", &WorkspaceFacts::default())
        .into_iter()
        .filter(|(code, _)| *code == "V13-ORM-SAVE-MODE")
        .map(|(_, line)| line)
        .collect()
}

#[test]
fn every_model_without_exactly_one_save_mode_must_change_at_its_name() {
    let source = r#"
use rullst::db::{Orm, RowSnapshot};

#[derive(Debug, Clone, Orm)]
#[orm(table = "profiles")]
pub struct Profile { pub id: i32, pub xp: i32 }

#[derive(rullst::db::Orm)]
pub struct Tracked {
    pub id: i32,
    #[sqlx(skip)]
    pub snapshot: rullst::orm::RowSnapshot,
}

#[derive(Orm)]
#[orm(table = "logs", full_row_save)]
pub struct Log { pub id: i32 }

#[derive(Orm)]
#[orm(table = "events")]
#[orm(full_row_save)]
pub struct Event { pub id: i32 }

#[derive(Orm)]
#[orm(full_row_save)]
pub struct Both {
    pub id: i32,
    #[sqlx(skip)]
    pub snapshot: RowSnapshot,
}

#[derive(Debug, Clone)]
pub struct NotAModel { pub id: i32 }
"#;
    assert_eq!(save_mode_lines(source), [6, 26]);
}

#[test]
fn turso_models_must_declare_full_row_saves_without_a_row_snapshot() {
    let source = r#"
#[derive(Orm)]
#[orm(table = "notes", backend = "turso")]
pub struct Note { pub id: i64 }

#[derive(TursoModel)]
#[orm(table = "tags", full_row_save)]
pub struct Tag { pub id: i64 }

#[derive(rullst_orm::TursoModel)]
pub struct Plain { pub id: i64 }

#[derive(Orm)]
#[orm(table = "drafts", backend = "turso", full_row_save)]
pub struct Draft {
    pub id: i64,
    #[sqlx(skip)]
    pub snapshot: RowSnapshot,
}
"#;
    assert_eq!(save_mode_lines(source), [4, 11, 15]);
}

#[test]
fn only_an_unparameterized_row_snapshot_type_counts() {
    let source = r#"
#[derive(Orm)]
pub struct Generic { pub id: i32, pub snapshot: RowSnapshot<u8> }

#[derive(Orm)]
pub struct Qualified { pub id: i32, pub snapshot: <T as Trait>::RowSnapshot }

#[derive(Orm)]
pub struct Reference { pub id: i32, pub snapshot: &'static RowSnapshot }
"#;
    assert_eq!(save_mode_lines(source), [3, 6, 9]);
}

#[test]
fn the_rule_belongs_to_the_explicit_save_mode_row() {
    let rule = &catalog::ORM_SAVE_MODE;
    assert_eq!(rule.code, "V13-ORM-SAVE-MODE");
    assert_eq!(rule.kind, FindingKind::MustChange);
    assert_eq!(rule.introduced, "13.0.0-alpha.6");
    assert_eq!(rule.row, "ORM explicit save mode");
    assert!(
        rule.guidance
            .contains("#[sqlx(skip)] pub snapshot: RowSnapshot,")
    );
    assert!(rule.guidance.contains("`full_row_save`"));
}
