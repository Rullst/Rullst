use rullst::db::{Orm, FromRow};
use rullst::nexus::{NexusModel, FieldMeta, FieldKind};
#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "categories")]
pub struct Category {
    pub id: i32,
    pub name: String,
}
impl NexusModel for Category {
    fn nexus_table() -> &'static str { "categories" }
    fn nexus_label() -> &'static str { "Categories" }
    fn nexus_icon() -> &'static str { "📁" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "name", label: "Name", kind: FieldKind::Text, hidden: false, readonly: false },
        ]
    }
}
