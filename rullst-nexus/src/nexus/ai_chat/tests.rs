//! Offline assistant answers, schema summary and the chat handlers.

use super::*;
use crate::nexus::types::{FieldKind, FieldMeta, RegistryEntry};

fn make_test_state() -> NexusState {
    let entry = RegistryEntry {
        table: "products",
        label: "Products",
        icon: "&#128196;",
        pk: "id",
        pk_columns: Vec::new(),
        tenant_column: None,
        fields: vec![
            FieldMeta::new("id", "ID", FieldKind::Number),
            FieldMeta::new("title", "Title", FieldKind::Text),
        ],
    };

    NexusState {
        registry: Arc::new(vec![entry]),
        brand: Arc::new("Admin Panel".to_string()),
        audit_policy: crate::nexus::NexusAuditPolicy::Disabled,
    }
}

#[test]
fn test_detect_ai_provider_default() {
    let (_has_prov, name) = detect_ai_provider();
    assert!(!name.is_empty());
}

#[test]
fn test_generate_smart_nexus_ai_response() {
    let state = make_test_state();

    // 1. Setup guide
    let resp_setup = generate_smart_nexus_ai_response("How to configure OpenAI provider?", &state);
    assert!(resp_setup.contains("Universal LLM Setup Guide"));
    assert!(resp_setup.contains("GEMINI_API_KEY"));

    // 2. Count query for specific table
    let resp_count = generate_smart_nexus_ai_response("How many products are registered?", &state);
    assert!(resp_count.contains("SELECT COUNT(*) AS total_products FROM products;"));

    // 3. Suggested query for specific table
    let resp_query = generate_smart_nexus_ai_response("Show products", &state);
    assert!(resp_query.contains("SELECT id, title FROM products"));

    // 4. General count queries across all models
    let resp_general_count = generate_smart_nexus_ai_response("Count total records", &state);
    assert!(resp_general_count.contains("total_products"));

    // 5. Help fallback
    let resp_fallback = generate_smart_nexus_ai_response("What can you do?", &state);
    assert!(
        resp_fallback.contains("analyzed your query") || resp_fallback.contains("example query")
    );
}

#[test]
fn schema_metadata_cannot_inject_markup_or_sql_into_fallback_output() {
    let state = NexusState {
        registry: Arc::new(vec![RegistryEntry {
            table: "products<script>",
            label: "<img src=x onerror=alert(1)>",
            icon: "📄",
            pk: "id",
            pk_columns: Vec::new(),
            tenant_column: None,
            fields: vec![FieldMeta::new(
                "title FROM secrets;--",
                "Title",
                FieldKind::Text,
            )],
        }]),
        brand: Arc::new("Admin".to_string()),
        audit_policy: crate::nexus::NexusAuditPolicy::Disabled,
    };

    let response = generate_smart_nexus_ai_response("list every table", &state);
    assert!(!response.contains("<script>"));
    assert!(!response.contains("<img"));
    assert!(!response.contains("FROM secrets"));
    assert!(response.contains("&lt;img"));
    assert!(response.contains("productsscript"));
}

#[tokio::test]
async fn test_nexus_chat_handlers() {
    let state = Arc::new(make_test_state());

    // Test Chat Page GET
    let headers = axum::http::HeaderMap::new();
    let page_html = nexus_chat_page(State(state.clone()), headers).await.0;
    assert!(page_html.contains("AI Query Assistant"));
    assert!(page_html.contains("nexus-chat-form"));

    // Test Chat Query POST
    let form_req = axum::extract::Form(ChatRequest {
        message: "Show products table".to_string(),
    });
    let query_html = nexus_chat_query(State(state), form_req).await.0;
    assert!(query_html.contains("nexus-chat-user"));
    assert!(query_html.contains("nexus-chat-assistant"));
    assert!(query_html.contains("products"));
}

#[test]
fn every_setup_keyword_answers_with_the_setup_guide() {
    let state = make_test_state();
    for keyword in [
        "configure",
        "setup",
        "provider",
        "env",
        "key",
        "deepseek",
        "qwen",
        "kimi",
    ] {
        let answer = generate_smart_nexus_ai_response(keyword, &state);
        assert!(answer.contains("Universal LLM Setup Guide"), "{keyword}");
    }
}

#[test]
fn a_model_is_matched_by_table_or_label_and_counted_by_any_count_keyword() {
    let mut state = make_test_state();
    let mut entry = state.registry[0].clone();
    entry.label = "Catalogue";
    state.registry = Arc::new(vec![entry]);
    let suggested = generate_smart_nexus_ai_response("catalogue", &state);
    assert!(
        suggested.contains("SELECT id, title FROM products ORDER BY id DESC LIMIT 20;"),
        "{suggested}"
    );
    for keyword in ["count", "how many", "total", "record"] {
        let answer = generate_smart_nexus_ai_response(&format!("products {keyword}"), &state);
        assert!(
            answer.contains("SELECT COUNT(*) AS total_products FROM products;"),
            "{keyword}"
        );
    }
}

#[test]
fn unmatched_questions_route_by_keyword() {
    let state = make_test_state();
    for keyword in ["count", "how many", "record", "total"] {
        let answer = generate_smart_nexus_ai_response(keyword, &state);
        assert!(
            answer.contains("Total Record Count Queries for Database Models"),
            "{keyword}"
        );
    }
    for keyword in ["table", "schema", "list", "show"] {
        let answer = generate_smart_nexus_ai_response(keyword, &state);
        assert!(answer.contains("Registered Database Models"), "{keyword}");
    }
}

#[test]
fn the_schema_summary_lists_each_model_with_typed_columns() {
    assert_eq!(
        schema_summary(&make_test_state()),
        "* products: id (number), title (text)"
    );
}

#[test]
fn provider_detection_reports_the_configured_providers() {
    let expected = match rullst_ai::AutoAiConfig::from_env() {
        Ok(config) if !config.provider_names().is_empty() => {
            (true, config.provider_names().join(" → "))
        }
        Ok(_) => (false, "Offline Embedded Intelligence".to_string()),
        Err(_) => (false, "AI configuration incomplete or invalid".to_string()),
    };
    assert_eq!(detect_ai_provider(), expected);
}

#[tokio::test]
async fn without_a_provider_the_chat_answers_offline() {
    if rullst_ai::AutoAiConfig::from_env().is_ok_and(|config| !config.provider_names().is_empty()) {
        return;
    }
    let answer = nexus_chat_query(
        State(Arc::new(make_test_state())),
        axum::extract::Form(ChatRequest {
            message: "show".to_string(),
        }),
    )
    .await
    .0;
    assert!(answer.contains("Registered Database Models"), "{answer}");
}
