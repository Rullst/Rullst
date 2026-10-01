use rullst::db::{FromRow, Orm};
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
