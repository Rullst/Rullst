// Versioned lesson availability and prerequisite persistence templates.

pub fn get_files() -> Vec<(&'static str, String)> {
    vec![
        (
            "src/models/lesson_release_rule.rs",
            LESSON_RELEASE_RULE.to_string(),
        ),
        (
            "src/migrations/m20260829000000_add_lesson_availability.rs",
            AVAILABILITY_MIGRATION.to_string(),
        ),
    ]
}

const LESSON_RELEASE_RULE: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "lesson_release_rules")]
pub struct LessonReleaseRule {
    pub id: i32,
    pub lesson_id: i32,
    pub ruleset_version: String,
    pub release_at_epoch: i64,
    pub expire_at_epoch: i64,
    pub prerequisite_lesson_id: i32,
    pub required_progress_percent: i32,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for LessonReleaseRule {
    fn nexus_table() -> &'static str { "lesson_release_rules" }
    fn nexus_label() -> &'static str { "Lesson Release Rules" }
    fn nexus_icon() -> &'static str { "🔐" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "lesson_id", label: "Lesson", kind: FieldKind::ForeignKey { table: "lessons", label_col: "title" }, hidden: false, readonly: true },
            FieldMeta { name: "ruleset_version", label: "Ruleset", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "release_at_epoch", label: "Release Epoch", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "expire_at_epoch", label: "Expire Epoch", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "prerequisite_lesson_id", label: "Prerequisite", kind: FieldKind::ForeignKey { table: "lessons", label_col: "title" }, hidden: false, readonly: true },
            FieldMeta { name: "required_progress_percent", label: "Required Progress %", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "status", label: "Status", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "created_at", label: "Created At", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "updated_at", label: "Updated At", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
"##;

const AVAILABILITY_MIGRATION: &str = r##"use rullst::db::{Orm, sqlx};
use rullst::db::async_trait;
use rullst::db::schema::{Migration, Schema};

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str { "m20260829000000_add_lesson_availability" }

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::create("lesson_release_rules", |table| {
            table.id();
            table.integer("lesson_id").not_null();
            table.string("ruleset_version").not_null();
            table.big_integer("release_at_epoch").not_null();
            table.big_integer("expire_at_epoch").not_null();
            table.integer("prerequisite_lesson_id").not_null();
            table.integer("required_progress_percent").not_null();
            table.string("status").not_null();
            table.timestamps();
        }).await?;
        let pool = Orm::pool()?;
        for statement in [
            "CREATE UNIQUE INDEX lesson_release_rules_version_unique ON lesson_release_rules(lesson_id, ruleset_version)",
            "CREATE INDEX lesson_release_rules_active_idx ON lesson_release_rules(lesson_id, status)",
        ] {
            sqlx::query(sqlx::AssertSqlSafe(statement)).execute(pool).await?;
        }
        for fixture in [
            "INSERT INTO lesson_release_rules (lesson_id, ruleset_version, release_at_epoch, expire_at_epoch, prerequisite_lesson_id, required_progress_percent, status, created_at, updated_at) VALUES (1, 'lesson-1-v1', 0, 0, 0, 0, 'active', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            "INSERT INTO lesson_release_rules (lesson_id, ruleset_version, release_at_epoch, expire_at_epoch, prerequisite_lesson_id, required_progress_percent, status, created_at, updated_at) VALUES (2, 'lesson-2-v1', 0, 0, 1, 100, 'active', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            "INSERT INTO lesson_release_rules (lesson_id, ruleset_version, release_at_epoch, expire_at_epoch, prerequisite_lesson_id, required_progress_percent, status, created_at, updated_at) VALUES (3, 'lesson-3-v1', 0, 0, 0, 0, 'active', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            "INSERT INTO lesson_release_rules (lesson_id, ruleset_version, release_at_epoch, expire_at_epoch, prerequisite_lesson_id, required_progress_percent, status, created_at, updated_at) VALUES (4, 'lesson-4-v1', 0, 0, 0, 0, 'active', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        ] {
            sqlx::query(sqlx::AssertSqlSafe(fixture)).execute(pool).await?;
        }
        Ok(())
    }

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::drop_if_exists("lesson_release_rules").await
    }
}
"##;

#[cfg(test)]
mod tests {
    use super::AVAILABILITY_MIGRATION;

    #[test]
    fn availability_policy_is_versioned_and_indexed() {
        assert!(AVAILABILITY_MIGRATION.contains("lesson_release_rules_version_unique"));
        assert!(AVAILABILITY_MIGRATION.contains("prerequisite_lesson_id"));
        assert!(AVAILABILITY_MIGRATION.contains("release_at_epoch"));
    }
}
