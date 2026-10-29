use rullst::db::async_trait;
use rullst::db::schema::{ColumnDefault, Migration, Schema};
use rullst::db::{Orm, sqlx};

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str {
        "m20260827000000_add_learning_access"
    }

    async fn up(&self) -> Result<(), rullst::orm::Error> {
        Schema::create("users", |table| {
            table.id();
            table.string("name").not_null();
            // MySQL/MariaDB cannot index a TEXT column without a prefix length, so
            // indexed strings are bounded VARCHAR columns on every driver.
            table.string("email").not_null().col_type = "VARCHAR(255)".to_string();
            table.string("password_hash").nullable();
            table.string("oauth_provider").nullable();
            table.string("oauth_id").nullable();
            table.timestamps();
            // Grants Nexus under the application's sign-in; see the User model.
            table
                .boolean("is_admin")
                .not_null()
                .default(ColumnDefault::Integer(0));
            // Bumped to revoke every login cookie of the account.
            table
                .big_integer("session_epoch")
                .not_null()
                .default(ColumnDefault::Integer(0));
        })
        .await?;

        Schema::create("enrollments", |table| {
            table.id();
            table.integer("user_id").not_null();
            table.integer("course_id").not_null();
            table.string("status").not_null().col_type = "VARCHAR(32)".to_string();
            table.timestamps();
        })
        .await?;

        Schema::create("lesson_progress", |table| {
            table.id();
            table.integer("user_id").not_null();
            table.integer("lesson_id").not_null();
            table.integer("progress_percent").not_null();
            table.integer("completed").not_null();
            table.timestamps();
        })
        .await?;

        Schema::create("lesson_progress_events", |table| {
            table.id();
            table.string("event_key").not_null().col_type = "VARCHAR(128)".to_string();
            table.integer("actor_user_id").not_null();
            table.integer("subject_user_id").not_null();
            table.integer("lesson_id").not_null();
            table.integer("previous_percent").not_null();
            table.integer("current_percent").not_null();
            table.string("event_kind").not_null();
            table.string("reason").not_null();
            table.timestamps();
        })
        .await?;

        let pool = Orm::pool()?;
        for statement in [
            "CREATE UNIQUE INDEX users_email_unique ON users(email)",
            "CREATE UNIQUE INDEX enrollments_user_course_unique ON enrollments(user_id, course_id)",
            "CREATE INDEX enrollments_course_status_idx ON enrollments(course_id, status)",
            "CREATE UNIQUE INDEX lesson_progress_user_lesson_unique ON lesson_progress(user_id, lesson_id)",
            "CREATE INDEX lesson_progress_lesson_idx ON lesson_progress(lesson_id)",
            // Idempotency keys are scoped to their learner: one learner's key can
            // never replay or block another learner's progress.
            "CREATE UNIQUE INDEX lesson_progress_events_subject_key_unique ON lesson_progress_events(subject_user_id, event_key)",
            "CREATE INDEX lesson_progress_events_subject_idx ON lesson_progress_events(subject_user_id, lesson_id, id)",
        ] {
            sqlx::query(sqlx::AssertSqlSafe(statement))
                .execute(pool)
                .await?;
        }
        Ok(())
    }

    async fn down(&self) -> Result<(), rullst::orm::Error> {
        Schema::drop_if_exists("lesson_progress_events").await?;
        Schema::drop_if_exists("lesson_progress").await?;
        Schema::drop_if_exists("enrollments").await?;
        Schema::drop_if_exists("users").await
    }
}
