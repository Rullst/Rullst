use rullst::db::{Orm, RowSnapshot};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};
#[derive(Debug, Clone, Orm)]
#[orm(table = "courses")]
pub struct Course {
    pub id: i32,
    pub category_id: i32,
    pub title: String,
    pub description: String,
    pub thumbnail: String,
    // Remembers the loaded values, so `save()` writes only the columns this
    // record changed. Build new records with `snapshot: RowSnapshot::default()`.
    #[sqlx(skip)]
    pub snapshot: RowSnapshot,
}
impl NexusModel for Course {
    fn nexus_table() -> &'static str {
        "courses"
    }
    fn nexus_label() -> &'static str {
        "Courses"
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
                "category_id",
                "Category",
                FieldKind::ForeignKey {
                    table: "categories",
                    label_col: "name",
                },
            ),
            FieldMeta::new("title", "Title", FieldKind::Text),
            FieldMeta::new("description", "Description", FieldKind::Textarea),
            FieldMeta::new("thumbnail", "Thumbnail URL", FieldKind::Url),
        ]
    }
}
