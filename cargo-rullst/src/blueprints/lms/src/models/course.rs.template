use rullst::db::{Orm, FromRow};
use rullst::nexus::{NexusModel, FieldMeta, FieldKind};
#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "courses")]
pub struct Course {
    pub id: i32,
    pub category_id: i32,
    pub title: String,
    pub description: String,
    pub thumbnail: String,
}
impl NexusModel for Course {
    fn nexus_table() -> &'static str { "courses" }
    fn nexus_label() -> &'static str { "Courses" }
    fn nexus_icon() -> &'static str { "🎓" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "category_id", label: "Category", kind: FieldKind::ForeignKey { table: "categories", label_col: "name" }, hidden: false, readonly: false },
            FieldMeta { name: "title", label: "Title", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "description", label: "Description", kind: FieldKind::Textarea, hidden: false, readonly: false },
            FieldMeta { name: "thumbnail", label: "Thumbnail URL", kind: FieldKind::Url, hidden: false, readonly: false },
        ]
    }
}
