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
            FieldMeta {
                name: "id",
                label: "ID",
                kind: FieldKind::Number,
                hidden: true,
                readonly: true,
            },
            FieldMeta {
                name: "category_id",
                label: "Category",
                kind: FieldKind::ForeignKey {
                    table: "categories",
                    label_col: "name",
                },
                hidden: false,
                readonly: false,
            },
            FieldMeta {
                name: "title",
                label: "Title",
                kind: FieldKind::Text,
                hidden: false,
                readonly: false,
            },
            FieldMeta {
                name: "description",
                label: "Description",
                kind: FieldKind::Textarea,
                hidden: false,
                readonly: false,
            },
            FieldMeta {
                name: "thumbnail",
                label: "Thumbnail URL",
                kind: FieldKind::Url,
                hidden: false,
                readonly: false,
            },
        ]
    }
}
