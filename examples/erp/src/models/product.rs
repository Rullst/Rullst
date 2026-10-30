use rullst::db::{Orm, RowSnapshot};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, Orm)]
#[orm(table = "products")]
pub struct Product {
    pub id: i32,
    pub name: String,
    pub sku: String,
    pub price: f64,
    pub stock: i32,
    // Remembers the loaded values, so `save()` writes only the columns this
    // record changed. Build new records with `snapshot: RowSnapshot::default()`.
    #[sqlx(skip)]
    pub snapshot: RowSnapshot,
}

impl NexusModel for Product {
    fn nexus_table() -> &'static str {
        "products"
    }
    fn nexus_label() -> &'static str {
        "Products"
    }
    fn nexus_icon() -> &'static str {
        "📦"
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
            FieldMeta {
                name: "sku",
                label: "SKU",
                kind: FieldKind::Text,
                hidden: false,
                readonly: false,
            },
            FieldMeta {
                name: "price",
                label: "Price",
                kind: FieldKind::Number,
                hidden: false,
                readonly: false,
            },
            FieldMeta {
                name: "stock",
                label: "Stock",
                kind: FieldKind::Number,
                hidden: false,
                readonly: false,
            },
        ]
    }
}
