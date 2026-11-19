use rullst::db::{Orm, RowSnapshot};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};
use rullst::orm::Timestamp;

#[derive(Debug, Clone, Orm)]
#[orm(table = "enrollments")]
pub struct Enrollment {
    pub id: i32,
    pub user_id: i32,
    pub course_id: i32,
    pub status: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    // Remembers the loaded values, so `save()` writes only the columns this
    // record changed. Build new records with `snapshot: RowSnapshot::default()`.
    #[sqlx(skip)]
    pub snapshot: RowSnapshot,
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
    fn nexus_table() -> &'static str {
        "enrollments"
    }
    fn nexus_label() -> &'static str {
        "Enrollments"
    }
    fn nexus_icon() -> &'static str {
        "🎓"
    }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta::new("id", "ID", FieldKind::Number)
                .hidden()
                .readonly(),
            FieldMeta::new(
                "user_id",
                "Learner",
                FieldKind::ForeignKey {
                    table: "users",
                    label_col: "email",
                },
            )
            .readonly(),
            FieldMeta::new(
                "course_id",
                "Course",
                FieldKind::ForeignKey {
                    table: "courses",
                    label_col: "title",
                },
            )
            .readonly(),
            FieldMeta::new("status", "Status", FieldKind::Text),
            FieldMeta::new("created_at", "Created At", FieldKind::DateTimeUtc).readonly(),
            FieldMeta::new("updated_at", "Updated At", FieldKind::DateTimeUtc).readonly(),
        ]
    }
}
