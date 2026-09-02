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
