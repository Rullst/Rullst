use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "course_modules")]
pub struct CourseModule {
    pub id: i32,
    pub course_id: i32,
    pub title: String,
    pub position: i32,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for CourseModule {
    fn nexus_table() -> &'static str { "course_modules" }
    fn nexus_label() -> &'static str { "Course Modules" }
    fn nexus_icon() -> &'static str { "🧭" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "course_id", label: "Course", kind: FieldKind::ForeignKey { table: "courses", label_col: "title" }, hidden: false, readonly: false },
            FieldMeta { name: "title", label: "Title", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "position", label: "Position", kind: FieldKind::Number, hidden: false, readonly: false },
            FieldMeta { name: "status", label: "Status", kind: FieldKind::Enum { options: vec!["draft", "published", "archived"] }, hidden: false, readonly: false },
            FieldMeta { name: "created_at", label: "Created At", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "updated_at", label: "Updated At", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
