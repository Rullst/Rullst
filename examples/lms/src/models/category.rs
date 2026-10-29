use rullst::db::{FromRow, Orm, RowSnapshot};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};
#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "categories")]
pub struct Category {
    pub id: i32,
    pub name: String,
    // Remembers the loaded values, so `save()` writes only the columns this
    // record changed. Build new records with `snapshot: RowSnapshot::default()`.
    #[sqlx(skip)]
    pub snapshot: RowSnapshot,
}
impl NexusModel for Category {
    fn nexus_table() -> &'static str {
        "categories"
    }
    fn nexus_label() -> &'static str {
        "Categories"
    }
    fn nexus_icon() -> &'static str {
        "📁"
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
                name: "name",
                label: "Name",
                kind: FieldKind::Text,
                hidden: false,
                readonly: false,
            },
        ]
    }
}
