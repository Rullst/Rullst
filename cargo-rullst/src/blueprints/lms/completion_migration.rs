// Reversible schema for derived course completion and certificate revocation.

pub fn get_files() -> Vec<(&'static str, String)> {
    vec![(
        "src/migrations/m20260903000000_add_course_completion.rs",
        COMPLETION_MIGRATION.to_string(),
    )]
}

const COMPLETION_MIGRATION: &str = r##"use rullst::db::{Orm, sqlx};
use rullst::db::async_trait;
use rullst::db::schema::{Migration, Schema};

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str { "m20260903000000_add_course_completion" }

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::create("course_completions", |table| {
            table.id(); table.string("completion_key").not_null();
            table.integer("subject_user_id").not_null(); table.integer("course_id").not_null();
            table.integer("course_version_id").not_null(); table.string("ruleset_version").not_null();
            table.big_integer("completed_at_epoch").not_null(); table.string("evidence_json").not_null();
            table.timestamps();
        }).await?;
        Schema::create("certificates", |table| {
            table.id(); table.string("certificate_key").not_null();
            table.integer("completion_id").not_null(); table.string("status").not_null();
            table.big_integer("issued_at_epoch").not_null(); table.string("revocation_key").nullable();
            table.integer("revoked_by").not_null(); table.big_integer("revoked_at_epoch").not_null();
            table.string("revocation_reason").not_null(); table.timestamps();
        }).await?;
        let pool = Orm::pool()?;
        for statement in [
            "CREATE UNIQUE INDEX course_completions_key_unique ON course_completions(completion_key)",
            "CREATE UNIQUE INDEX course_completions_subject_version_unique ON course_completions(subject_user_id, course_version_id)",
            "CREATE INDEX course_completions_course_subject_idx ON course_completions(course_id, subject_user_id)",
            "CREATE UNIQUE INDEX certificates_key_unique ON certificates(certificate_key)",
            "CREATE UNIQUE INDEX certificates_completion_unique ON certificates(completion_id)",
            "CREATE UNIQUE INDEX certificates_revocation_key_unique ON certificates(revocation_key)",
            "CREATE INDEX certificates_status_issued_idx ON certificates(status, issued_at_epoch)",
        ] { sqlx::query(sqlx::AssertSqlSafe(statement)).execute(pool).await?; }
        Ok(())
    }

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::drop_if_exists("certificates").await?;
        Schema::drop_if_exists("course_completions").await
    }
}
"##;

#[cfg(test)]
mod tests {
    use super::COMPLETION_MIGRATION;

    #[test]
    fn completion_schema_is_reversible_and_deduplicated() {
        assert!(COMPLETION_MIGRATION.contains("course_completions_subject_version_unique"));
        assert!(COMPLETION_MIGRATION.contains("certificates_completion_unique"));
        assert!(COMPLETION_MIGRATION.contains("certificates_revocation_key_unique"));
        assert!(COMPLETION_MIGRATION.contains("Schema::drop_if_exists(\"certificates\")"));
    }
}
