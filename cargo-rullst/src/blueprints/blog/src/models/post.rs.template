use rullst::db::{Orm, FromRow};
use rullst::nexus::{NexusModel, FieldMeta, FieldKind};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "posts")]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub slug: String,
    pub content: String,
}

impl NexusModel for Post {
    fn nexus_table() -> &'static str { "posts" }
    fn nexus_label() -> &'static str { "Posts" }
    fn nexus_icon() -> &'static str { "📝" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "title", label: "Title", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "slug", label: "Slug", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "content", label: "Content", kind: FieldKind::Textarea, hidden: false, readonly: false },
        ]
    }
}
