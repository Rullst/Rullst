use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "users")]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    #[orm(hidden)]
    pub password_hash: Option<String>,
    pub oauth_provider: Option<String>,
    pub oauth_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl User {
    pub async fn find_by_email(email: &str) -> Result<Option<Self>, rullst_orm::Error> {
        Self::query()
            .where_eq("email", email.to_owned())
            .first()
            .await
    }
}

impl NexusModel for User {
    fn nexus_table() -> &'static str { "users" }
    fn nexus_label() -> &'static str { "Learners" }
    fn nexus_icon() -> &'static str { "👥" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "name", label: "Name", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "email", label: "Email", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "password_hash", label: "Password Hash", kind: FieldKind::Text, hidden: true, readonly: true },
            FieldMeta { name: "oauth_provider", label: "OAuth Provider", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "oauth_id", label: "OAuth ID", kind: FieldKind::Text, hidden: true, readonly: true },
            FieldMeta { name: "created_at", label: "Created At", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "updated_at", label: "Updated At", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
