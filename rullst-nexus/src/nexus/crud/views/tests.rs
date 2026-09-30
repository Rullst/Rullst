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

#[tokio::test]
async fn password_columns_have_no_sort_link() {
    let entry = RegistryEntry {
        table: "accounts",
        label: "Accounts",
        icon: "A",
        pk: "id",
        tenant_column: None,
        fields: vec![
            FieldMeta::new("name", "Name", FieldKind::Text),
            FieldMeta::new("api_key", "API key", FieldKind::Password),
        ],
    };
    let html = render_table_view(&state(), &entry, 1, "", None, None, None).await;
    assert!(html.contains("sort_by=name"));
    assert!(!html.contains("sort_by=api_key"));
    assert!(html.contains("<th class=\"nexus-th\">API key</th>"));
}
