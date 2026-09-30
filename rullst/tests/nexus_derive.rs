#![cfg(all(feature = "nexus", feature = "orm"))]

use rullst::db::Nexus;
use rullst::nexus::{FieldKind, NexusModel};

#[derive(Nexus)]
#[allow(dead_code)]
#[nexus(
    table = "derived_articles",
    label = "Editorial Articles",
    icon = "📰",
    primary_key = "uuid"
)]
struct DerivedArticle {
    uuid: String,
    #[nexus(kind = "textarea", label = "Article body")]
    body: String,
    #[nexus(kind = "enum", options = "draft, published")]
    status: String,
    is_active: bool,
}

#[derive(Nexus)]
#[allow(dead_code)]
#[nexus(table = "tenant_articles", tenant = "organization_id")]
struct TenantArticle {
    id: i64,
    organization_id: String,
    title: String,
}

/// A real ORM model: Nexus must follow its skip and confidentiality markers.
#[derive(Clone, Debug, Default, rullst::db::Orm, rullst::db::FromRow, Nexus)]
#[allow(dead_code)]
#[orm(table = "nexus_patients")]
struct NexusPatient {
    id: i32,
    name: String,
    #[sqlx(default, skip)]
    display_name: String,
    #[orm(hidden)]
    reset_token: String,
    #[orm(encrypted)]
    diagnosis: String,
    cpf: Option<rullst::db::SecretString>,
    #[orm(masked)]
    api_token: String,
}

#[test]
fn derive_nexus_follows_orm_field_semantics() {
    let fields = NexusPatient::nexus_fields();
    assert!(fields.iter().all(|field| field.name != "display_name"));
    for sealed in ["reset_token", "diagnosis", "cpf"] {
        let field = fields
            .iter()
            .find(|field| field.name == sealed)
            .expect("protected field metadata");
        assert!(field.hidden && field.readonly, "{sealed}");
        assert_eq!(field.kind, FieldKind::Password, "{sealed}");
    }
    let masked = fields
        .iter()
        .find(|field| field.name == "api_token")
        .expect("masked field metadata");
    assert_eq!(masked.kind, FieldKind::Password);
    assert!(!masked.hidden);
    let name = fields
        .iter()
        .find(|field| field.name == "name")
        .expect("plain field metadata");
    assert_eq!(name.kind, FieldKind::Text);
}

#[test]
fn derive_nexus_generates_model_and_widget_metadata() {
    assert_eq!(DerivedArticle::nexus_table(), "derived_articles");
    assert_eq!(DerivedArticle::nexus_label(), "Editorial Articles");
    assert_eq!(DerivedArticle::nexus_icon(), "📰");
    assert_eq!(DerivedArticle::nexus_pk(), "uuid");

    let fields = DerivedArticle::nexus_fields();
    assert_eq!(fields.len(), 4);
    assert!(fields[0].hidden && fields[0].readonly);
    assert_eq!(fields[1].label, "Article body");
    assert_eq!(fields[1].kind, FieldKind::Textarea);
    assert_eq!(
        fields[2].kind,
        FieldKind::Enum {
            options: vec!["draft", "published"]
        }
    );
    assert_eq!(fields[3].kind, FieldKind::Boolean);
}

#[test]
fn derive_nexus_exposes_protected_tenant_metadata_through_the_facade() {
    assert_eq!(
        TenantArticle::nexus_tenant_column(),
        Some("organization_id")
    );
    let tenant = TenantArticle::nexus_fields()
        .into_iter()
        .find(|field| field.name == "organization_id")
        .expect("tenant field metadata");
    assert!(tenant.hidden && tenant.readonly);
    assert_eq!(tenant.kind, FieldKind::Text);
}
