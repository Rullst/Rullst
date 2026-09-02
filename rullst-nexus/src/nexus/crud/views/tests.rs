use super::*;
use std::sync::Arc;

fn state() -> NexusState {
    NexusState {
        registry: Arc::new(vec![]),
        brand: Arc::new("Nexus".to_string()),
        audit_policy: crate::nexus::NexusAuditPolicy::Disabled,
    }
}

#[tokio::test]
async fn table_view_escapes_metadata_and_only_offers_supported_batch_actions() {
    let entry = RegistryEntry {
        table: "articles",
        label: "<img src=x onerror=alert(1)>",
        icon: "📰",
        pk: "id",
        tenant_column: None,
        fields: vec![
            FieldMeta::new("title", "<script>alert(1)</script>", FieldKind::Text),
            FieldMeta::new("is_active", "Active", FieldKind::Boolean),
        ],
    };
    let html = render_table_view(
        &state(),
        &entry,
        1,
        "\"><svg/onload=alert(1)>",
        Some("title"),
        Some("asc"),
        None,
    )
    .await;

    assert!(!html.contains("<script>"));
    assert!(!html.contains("<img src=x"));
    assert!(!html.contains("<svg"));
    assert!(html.contains("&lt;script&gt;"));
    assert!(html.contains("value=\"deactivate\""));

    let without_active = RegistryEntry {
        fields: vec![FieldMeta::new("title", "Title", FieldKind::Text)],
        ..entry
    };
    let html = render_table_view(&state(), &without_active, 1, "", None, None, None).await;
    assert!(!html.contains("value=\"deactivate\""));
}
