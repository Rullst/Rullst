// Learning-domain persistence templates kept outside the blueprint orchestrator.

#[path = "roles.rs"]
mod roles;

pub fn get_files() -> Vec<(&'static str, String)> {
    let mut files = vec![
        ("src/models/user.rs", USER_MODEL.to_string()),
        ("src/models/enrollment.rs", ENROLLMENT_MODEL.to_string()),
        (
            "src/models/lesson_progress.rs",
            LESSON_PROGRESS_MODEL.to_string(),
        ),
        (
            "src/models/lesson_progress_event.rs",
            LESSON_PROGRESS_EVENT_MODEL.to_string(),
        ),
        (
            "src/migrations/m20260827000000_add_learning_access.rs",
            LEARNING_MIGRATION.to_string(),
        ),
    ];
    files.extend(roles::get_files());
    files
}

const USER_MODEL: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "users")]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    #[orm(hidden)]
    pub password_hash: Option<String>,
    pub oauth_provider: Option<String>,
    pub oauth_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl User {
    pub async fn find_by_email(email: &str) -> Result<Option<Self>, rullst_orm::Error> {
        Self::query()
            .where_eq("email", email.to_owned())
            .first()
            .await
    }
}

impl NexusModel for User {
    fn nexus_table() -> &'static str { "users" }
    fn nexus_label() -> &'static str { "Learners" }
    fn nexus_icon() -> &'static str { "👥" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "name", label: "Name", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "email", label: "Email", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "password_hash", label: "Password Hash", kind: FieldKind::Text, hidden: true, readonly: true },
            FieldMeta { name: "oauth_provider", label: "OAuth Provider", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "oauth_id", label: "OAuth ID", kind: FieldKind::Text, hidden: true, readonly: true },
            FieldMeta { name: "created_at", label: "Created At", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "updated_at", label: "Updated At", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
"##;

const ENROLLMENT_MODEL: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "enrollments")]
pub struct Enrollment {
    pub id: i32,
    pub user_id: i32,
    pub course_id: i32,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Enrollment {
    pub async fn active_for(
        user_id: i32,
        course_id: i32,
    ) -> Result<Option<Self>, rullst_orm::Error> {
        Self::query()
            .where_eq("user_id", user_id)
            .where_eq("course_id", course_id)
            .where_eq("status", "active")
            .first()
            .await
    }
}

impl NexusModel for Enrollment {
    fn nexus_table() -> &'static str { "enrollments" }
    fn nexus_label() -> &'static str { "Enrollments" }
    fn nexus_icon() -> &'static str { "🎓" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "user_id", label: "Learner", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "course_id", label: "Course", kind: FieldKind::ForeignKey { table: "courses", label_col: "title" }, hidden: false, readonly: true },
            FieldMeta { name: "status", label: "Status", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "created_at", label: "Created At", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "updated_at", label: "Updated At", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
"##;

const LESSON_PROGRESS_MODEL: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "lesson_progress")]
pub struct LessonProgress {
    pub id: i32,
    pub user_id: i32,
    pub lesson_id: i32,
    pub progress_percent: i32,
    pub completed: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl LessonProgress {
    pub async fn for_learner(
        user_id: i32,
        lesson_id: i32,
    ) -> Result<Option<Self>, rullst_orm::Error> {
        Self::query()
            .where_eq("user_id", user_id)
            .where_eq("lesson_id", lesson_id)
            .first()
            .await
    }
}

impl NexusModel for LessonProgress {
    fn nexus_table() -> &'static str { "lesson_progress" }
    fn nexus_label() -> &'static str { "Lesson Progress" }
    fn nexus_icon() -> &'static str { "📈" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "user_id", label: "Learner", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "lesson_id", label: "Lesson", kind: FieldKind::ForeignKey { table: "lessons", label_col: "title" }, hidden: false, readonly: true },
            FieldMeta { name: "progress_percent", label: "Progress %", kind: FieldKind::Number, hidden: false, readonly: false },
            FieldMeta { name: "completed", label: "Completed", kind: FieldKind::Number, hidden: false, readonly: false },
            FieldMeta { name: "created_at", label: "Created At", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "updated_at", label: "Updated At", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
"##;

const LESSON_PROGRESS_EVENT_MODEL: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "lesson_progress_events")]
pub struct LessonProgressEvent {
    pub id: i32,
    pub event_key: String,
    pub actor_user_id: i32,
    pub subject_user_id: i32,
    pub lesson_id: i32,
    pub previous_percent: i32,
    pub current_percent: i32,
    pub event_kind: String,
    pub reason: String,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for LessonProgressEvent {
    fn nexus_table() -> &'static str { "lesson_progress_events" }
    fn nexus_label() -> &'static str { "Progress Audit" }
    fn nexus_icon() -> &'static str { "🧾" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "event_key", label: "Event Key", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "actor_user_id", label: "Actor", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "subject_user_id", label: "Learner", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "lesson_id", label: "Lesson", kind: FieldKind::ForeignKey { table: "lessons", label_col: "title" }, hidden: false, readonly: true },
            FieldMeta { name: "previous_percent", label: "Previous %", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "current_percent", label: "Current %", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "event_kind", label: "Kind", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "reason", label: "Reason", kind: FieldKind::Textarea, hidden: false, readonly: true },
            FieldMeta { name: "created_at", label: "Created At", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "updated_at", label: "Updated At", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
"##;

const LEARNING_MIGRATION: &str = r##"use rullst::db::{Orm, sqlx};
use rullst::db::async_trait;
use rullst::db::schema::{Migration, Schema};

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str {
        "m20260827000000_add_learning_access"
    }

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::create("users", |table| {
            table.id();
            table.string("name").not_null();
            table.string("email").not_null();
            table.string("password_hash").nullable();
            table.string("oauth_provider").nullable();
            table.string("oauth_id").nullable();
            table.timestamps();
        }).await?;

        Schema::create("enrollments", |table| {
            table.id();
            table.integer("user_id").not_null();
            table.integer("course_id").not_null();
            table.string("status").not_null();
            table.timestamps();
        }).await?;

        Schema::create("lesson_progress", |table| {
            table.id();
            table.integer("user_id").not_null();
            table.integer("lesson_id").not_null();
            table.integer("progress_percent").not_null();
            table.integer("completed").not_null();
            table.timestamps();
        }).await?;

        Schema::create("lesson_progress_events", |table| {
            table.id();
            table.string("event_key").not_null();
            table.integer("actor_user_id").not_null();
            table.integer("subject_user_id").not_null();
            table.integer("lesson_id").not_null();
            table.integer("previous_percent").not_null();
            table.integer("current_percent").not_null();
            table.string("event_kind").not_null();
            table.string("reason").not_null();
            table.timestamps();
        }).await?;

        let pool = Orm::pool()?;
        for statement in [
            "CREATE UNIQUE INDEX users_email_unique ON users(email)",
            "CREATE UNIQUE INDEX enrollments_user_course_unique ON enrollments(user_id, course_id)",
            "CREATE INDEX enrollments_course_status_idx ON enrollments(course_id, status)",
            "CREATE UNIQUE INDEX lesson_progress_user_lesson_unique ON lesson_progress(user_id, lesson_id)",
            "CREATE INDEX lesson_progress_lesson_idx ON lesson_progress(lesson_id)",
            "CREATE UNIQUE INDEX lesson_progress_events_key_unique ON lesson_progress_events(event_key)",
            "CREATE INDEX lesson_progress_events_subject_idx ON lesson_progress_events(subject_user_id, lesson_id, created_at)",
        ] {
            sqlx::query(sqlx::AssertSqlSafe(statement)).execute(pool).await?;
        }
        Ok(())
    }

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::drop_if_exists("lesson_progress_events").await?;
        Schema::drop_if_exists("lesson_progress").await?;
        Schema::drop_if_exists("enrollments").await?;
        Schema::drop_if_exists("users").await
    }
}
"##;
