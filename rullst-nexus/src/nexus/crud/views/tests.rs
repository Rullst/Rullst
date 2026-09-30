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

#[tokio::test]
async fn live_search_keeps_the_sort_and_rebuilds_pagination_outside_the_bulk_form() {
    let entry = RegistryEntry {
        table: "articles",
        label: "Articles",
        icon: "A",
        pk: "id",
        tenant_column: None,
        fields: vec![
            FieldMeta::new("id", "ID", FieldKind::Number).readonly(),
            FieldMeta::new("name", "Name", FieldKind::Text),
        ],
    };
    let html = render_table_view(
        &state(),
        &entry,
        3,
        "smith",
        Some("name"),
        Some("asc"),
        None,
    )
    .await;

    let bulk_form = html
        .split("<form id=\"batch-form-articles\"")
        .nth(1)
        .and_then(|rest| rest.split("</form>").next())
        .expect("bulk form");
    // Enter in the search box must not submit the bulk form.
    assert!(!bulk_form.contains("name=\"q\""));
    // The live-search region holds the rows, sort links and pagination.
    let region = bulk_form
        .split("<div id=\"nexus-table-region\">")
        .nth(1)
        .expect("table region");
    assert!(region.contains("sort_by=name&amp;order=desc&amp;q=smith"));
    assert!(region.contains("?page=4&amp;q=smith&amp;sort_by=name&amp;order=asc"));

    let search_form = html
        .split("<form class=\"nexus-search-wrap\"")
        .nth(1)
        .and_then(|rest| rest.split("</form>").next())
        .expect("search form");
    assert!(search_form.contains("method=\"GET\" action=\"/nexus/table/articles\""));
    assert!(search_form.contains("name=\"q\" value=\"smith\""));
    assert!(search_form.contains("<input type=\"hidden\" name=\"sort_by\" value=\"name\" />"));
    assert!(search_form.contains("<input type=\"hidden\" name=\"order\" value=\"asc\" />"));
    // The live search requests the full view (not the rows-only fragment),
    // keeps the sort through the form and swaps only the table region.
    assert!(search_form.contains("hx-get=\"/nexus/table/articles\" hx-trigger="));
    assert!(!search_form.contains("/search"));
    assert!(search_form.contains("hx-include=\"closest form\""));
    assert!(search_form.contains("hx-select=\"#nexus-table-region\""));
    assert!(search_form.contains("hx-push-url=\"true\""));

    assert!(html.contains("form=\"batch-form-articles\""));
}

#[test]
fn saving_a_record_refreshes_the_current_query() {
    let script = include_str!("../../../../assets/nexus.js");
    assert!(script.contains("window.location.pathname + window.location.search"));
}
