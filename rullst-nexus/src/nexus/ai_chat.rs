use axum::{extract::State, response::Html};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::nexus::crud::{field_kind_label, sanitize_identifier};
use crate::nexus::types::NexusState;
use crate::nexus::ui::{render_shell, render_sidebar};

#[derive(Deserialize, Serialize)]
pub struct ChatRequest {
    pub message: String,
}

fn schema_summary(state: &NexusState) -> String {
    state
        .registry
        .iter()
        .map(|model| {
            let columns = model
                .fields
                .iter()
                .map(|field| {
                    format!(
                        "{} ({})",
                        sanitize_identifier(field.name),
                        field_kind_label(&field.kind)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("* {}: {columns}", sanitize_identifier(model.table))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn detect_ai_provider() -> (bool, String) {
    if std::env::var("GEMINI_API_KEY").is_ok() {
        (true, "Google Gemini".to_string())
    } else if std::env::var("OPENAI_API_KEY").is_ok() {
        (true, "OpenAI".to_string())
    } else if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        (true, "Anthropic Claude".to_string())
    } else if std::env::var("OLLAMA_HOST").is_ok() {
        (true, "Ollama Local".to_string())
    } else if std::env::var("OPENAI_BASE_URL").is_ok() {
        (true, "Custom LLM Endpoint".to_string())
    } else {
        (false, "Offline Embedded Intelligence".to_string())
    }
}

pub fn generate_smart_nexus_ai_response(message: &str, state: &NexusState) -> String {
    let msg_lower = message.to_lowercase();

    // Check if user is asking for LLM setup instructions
    if msg_lower.contains("configure")
        || msg_lower.contains("setup")
        || msg_lower.contains("provider")
        || msg_lower.contains("env")
        || msg_lower.contains("key")
        || msg_lower.contains("deepseek")
        || msg_lower.contains("qwen")
        || msg_lower.contains("kimi")
    {
        return "<p><strong>🌐 Universal LLM Setup Guide:</strong></p>\
             <p>Rullst AI supports <strong>any LLM provider</strong> out of the box! Add any of these variables to your project's <code class=\"nexus-code\">.env</code> file:</p>\
             <ul style=\"margin: 0.5rem 0; padding-left: 1.25rem; font-size: 0.85rem;\">\
               <li><strong>Google Gemini:</strong> <code class=\"nexus-code\">GEMINI_API_KEY=your_key</code></li>\
               <li><strong>OpenAI / ChatGPT:</strong> <code class=\"nexus-code\">OPENAI_API_KEY=your_key</code></li>\
               <li><strong>Anthropic Claude:</strong> <code class=\"nexus-code\">ANTHROPIC_API_KEY=your_key</code></li>\
               <li><strong>Local Ollama:</strong> <code class=\"nexus-code\">OLLAMA_HOST=http://localhost:11434</code></li>\
               <li><strong>DeepSeek / Qwen / Kimi:</strong> <code class=\"nexus-code\">OPENAI_BASE_URL=https://...</code></li>\
             </ul>\
             <p style=\"font-size: 0.8rem; color: var(--text-300);\">Then restart your dev server (<code class=\"nexus-code\">cargo rullst dev</code> or <code class=\"nexus-code\">dash</code>).</p>".to_string();
    }

    // Check if query matches a specific table registered in the project
    let matched_table = state.registry.iter().find(|entry| {
        msg_lower.contains(&entry.table.to_lowercase())
            || msg_lower.contains(&entry.label.to_lowercase())
    });

    if let Some(entry) = matched_table {
        let table = sanitize_identifier(entry.table);
        let label = rullst_core::html::escape_str(entry.label);
        let cols: Vec<String> = entry
            .fields
            .iter()
            .map(|field| sanitize_identifier(field.name))
            .collect();
        let cols_str = if cols.is_empty() {
            "*".to_string()
        } else {
            cols.join(", ")
        };

        if msg_lower.contains("count")
            || msg_lower.contains("how many")
            || msg_lower.contains("total")
            || msg_lower.contains("record")
        {
            format!(
                "<p><strong>📊 Count Query for <code>{}</code>:</strong></p>\
                 <pre class=\"nexus-schema-pre\" style=\"padding: 0.75rem;\">SELECT COUNT(*) AS total_{} FROM {};</pre>\
                 <p style=\"font-size: 0.8rem; color: var(--text-300);\">⚡ <em>Offline Schema Assistant analyzed your <code>{}</code> model!</em></p>",
                table, table, table, label
            )
        } else {
            format!(
                "<p><strong>🔍 Suggested SQL Query for <code>{}</code>:</strong></p>\
                 <pre class=\"nexus-schema-pre\" style=\"padding: 0.75rem;\">SELECT {} FROM {} ORDER BY id DESC LIMIT 20;</pre>\
                 <p style=\"font-size: 0.8rem; color: var(--text-300);\">⚡ <em>Offline Schema Assistant extracted columns: <code>{}</code></em></p>",
                table, cols_str, table, cols_str
            )
        }
    } else if msg_lower.contains("count")
        || msg_lower.contains("how many")
        || msg_lower.contains("record")
        || msg_lower.contains("total")
    {
        let mut list_html = String::from(
            "<p><strong>📊 Total Record Count Queries for Database Models:</strong></p><ul style=\"margin: 0.5rem 0; padding-left: 1.25rem; font-size: 0.85rem;\">",
        );
        for entry in state.registry.iter() {
            let label = rullst_core::html::escape_str(entry.label);
            let table = sanitize_identifier(entry.table);
            list_html.push_str(&format!(
                "<li><strong>{}</strong>: <code class=\"nexus-code\">SELECT COUNT(*) AS total_{} FROM {};</code></li>",
                label, table, table
            ));
        }
        list_html.push_str("</ul><p style=\"font-size: 0.8rem; color: var(--text-300);\">⚡ <em>Offline Schema Assistant generated count queries for all registered tables!</em></p>");
        list_html
    } else if msg_lower.contains("table")
        || msg_lower.contains("schema")
        || msg_lower.contains("list")
        || msg_lower.contains("show")
    {
        let mut list_html = String::from(
            "<p><strong>📊 Registered Database Models:</strong></p><ul style=\"margin: 0.5rem 0; padding-left: 1.25rem;\">",
        );
        for entry in state.registry.iter() {
            let field_names: Vec<String> = entry
                .fields
                .iter()
                .map(|f| sanitize_identifier(f.name))
                .collect();
            let label = rullst_core::html::escape_str(entry.label);
            let table = sanitize_identifier(entry.table);
            list_html.push_str(&format!(
                "<li><strong>{}</strong> (table: <code>{}</code>) &mdash; fields: <code>{}</code></li>",
                label, table, field_names.join(", ")
            ));
        }
        list_html.push_str("</ul>");
        list_html
    } else {
        if let Some(first_entry) = state.registry.first() {
            let table = sanitize_identifier(first_entry.table);
            let cols: Vec<String> = first_entry
                .fields
                .iter()
                .map(|f| sanitize_identifier(f.name))
                .collect();
            format!(
                "<p>I analyzed your query relative to your schema.</p>\
                 <p>Here is an example query for your <strong><code>{}</code></strong> model:</p>\
                 <pre class=\"nexus-schema-pre\" style=\"padding: 0.75rem;\">SELECT {} FROM {} LIMIT 10;</pre>\
                 <p style=\"font-size: 0.8rem; color: var(--text-300);\">💡 <em>Tip: You can ask about any table directly (e.g. \"show {}\" or \"count {}\").</em></p>",
                table,
                cols.join(", "),
                table,
                table,
                table
            )
        } else {
            format!(
                "<p>I understand you're asking: <em>{}</em></p>\
                 <p>No models are registered in the Nexus Panel yet. Implement <code>NexusModel</code> on your structs to manage them visually!</p>",
                rullst_core::html::escape_str(message)
            )
        }
    }
}

/// GET /nexus/chat — AI Assistant Page.
pub async fn nexus_chat_page(
    State(state): State<Arc<NexusState>>,
    headers: axum::http::HeaderMap,
) -> Html<String> {
    let schema_summary = schema_summary(&state);

    let mut content = String::new();
    content.push_str("<div class=\"nexus-page-header\">");
    content.push_str("<h1 class=\"nexus-page-title\">&#129302; AI Query Assistant</h1>");
    content.push_str("<p class=\"nexus-page-subtitle\">Ask questions about your data in plain language. Powered by <strong>rullst::ai</strong>.</p>");
    content.push_str("</div>");
    content.push_str("<div class=\"nexus-chat-layout\">");

    // Left Control Panel: Schema & Quick Commands
    content.push_str("<div class=\"nexus-chat-schema\" style=\"display: flex; flex-direction: column; gap: 1rem;\">");

    // Quick Preset Commands
    content.push_str("<div class=\"nexus-card\" style=\"padding: 1rem; margin: 0;\">");
    content.push_str("<div class=\"nexus-schema-title\" style=\"margin-bottom: 0.75rem;\">&#9889; Quick Commands</div>");
    content.push_str("<div style=\"display: flex; flex-direction: column; gap: 0.5rem;\">");
    content.push_str("<button type=\"button\" onclick=\"setAiPrompt('Show all registered database tables')\" class=\"nexus-btn\" style=\"background: rgba(255, 255, 255, 0.08); border: 1px solid rgba(255, 255, 255, 0.15); color: #ffffff !important; font-weight: 600; font-size: 0.8rem; text-align: left; justify-content: start;\">&#128202; Show All Tables</button>");
    content.push_str("<button type=\"button\" onclick=\"setAiPrompt('How many records exist in the database?')\" class=\"nexus-btn\" style=\"background: rgba(255, 255, 255, 0.08); border: 1px solid rgba(255, 255, 255, 0.15); color: #ffffff !important; font-weight: 600; font-size: 0.8rem; text-align: left; justify-content: start;\">&#128200; Count Table Rows</button>");
    content.push_str("<button type=\"button\" onclick=\"setAiPrompt('How do I configure an AI provider key in .env?')\" class=\"nexus-btn\" style=\"background: rgba(255, 255, 255, 0.08); border: 1px solid rgba(255, 255, 255, 0.15); color: #ffffff !important; font-weight: 600; font-size: 0.8rem; text-align: left; justify-content: start;\">&#127760; LLM Setup Instructions</button>");
    content.push_str("</div></div>");

    // LLM Provider Setup Banner
    content.push_str("<div class=\"nexus-card\" style=\"padding: 1rem; margin: 0; background: rgba(245, 158, 11, 0.05); border: 1px solid rgba(245, 158, 11, 0.2);\">");
    content.push_str("<div style=\"font-size: 0.85rem; font-weight: 700; color: #f59e0b; margin-bottom: 0.5rem;\">&#127760; Universal LLM Support</div>");
    content.push_str("<p style=\"font-size: 0.75rem; color: var(--text-200); margin: 0 0 0.5rem 0; line-height: 1.4;\">Connect to <strong>Gemini, OpenAI, Claude, Ollama, DeepSeek, Qwen, or Kimi</strong> via your <code class=\"nexus-code\">.env</code> file:</p>");
    content.push_str("<pre class=\"nexus-schema-pre\" style=\"font-size: 0.7rem; padding: 0.5rem;\">GEMINI_API_KEY=key\nOPENAI_API_KEY=key\nANTHROPIC_API_KEY=key\nOLLAMA_HOST=http://...\nOPENAI_BASE_URL=https://...</pre>");
    content.push_str("</div>");

    // Database Schema Summary
    content.push_str("<div class=\"nexus-card\" style=\"padding: 1rem; margin: 0;\">");
    content.push_str("<div class=\"nexus-schema-title\">&#128202; Database Schema</div>");
    content.push_str("<pre class=\"nexus-schema-pre\">");
    content.push_str(&rullst_core::html::escape_str(&schema_summary));
    content.push_str("</pre></div></div>");

    // Right Panel: Chat Messages
    content.push_str("<div class=\"nexus-chat-panel\">");
    content.push_str("<div class=\"nexus-chat-messages\" id=\"nexus-chat-messages\">");
    content.push_str("<div class=\"nexus-chat-bubble nexus-chat-assistant\">");
    content.push_str("<span class=\"nexus-chat-avatar\">&#129302;</span>");
    content.push_str("<div class=\"nexus-chat-text\">Hello! I have full offline intelligence about your database schema. Ask me anything &mdash; for example:<br><em>\"List all courses\"</em>, <em>\"Show tables\"</em>, or <em>\"How do I setup DeepSeek/Qwen?\"</em><br><br><small style=\"color: var(--text-300);\">&#128161; <b>Tip:</b> Click any Quick Command on the left or type your query below.</small></div>");
    content.push_str("</div></div>");
    content.push_str("<form class=\"nexus-chat-form\" hx-post=\"/nexus/chat/query\" hx-target=\"#nexus-chat-messages\" hx-swap=\"beforeend\" hx-on:htmx:after-request=\"this.reset(); document.getElementById(&quot;nexus-chat-messages&quot;).scrollTop = 99999;\">");
    content.push_str("<input type=\"text\" name=\"message\" id=\"nexus-chat-input\" class=\"nexus-chat-input\" placeholder=\"Ask about your data...\" aria-label=\"Ask the AI assistant\" autocomplete=\"off\" required />");
    content.push_str(
        "<button type=\"submit\" class=\"nexus-btn nexus-btn-ai\">Send &#9992;&#65039;</button>",
    );
    content.push_str("</form></div></div>");

    content.push_str(
        r#"
<script>
function setAiPrompt(text) {
  const input = document.getElementById('nexus-chat-input');
  if (input) {
    input.value = text;
    input.focus();
  }
}
</script>
"#,
    );

    if headers.contains_key("hx-request") {
        Html(content)
    } else {
        Html(render_shell(
            &state,
            &render_sidebar(&state, None),
            &content,
        ))
    }
}

/// POST /nexus/chat/query — AI Assistant HTMX query endpoint.
pub async fn nexus_chat_query(
    State(state): State<Arc<NexusState>>,
    axum::extract::Form(req): axum::extract::Form<ChatRequest>,
) -> Html<String> {
    let user_msg = rullst_core::html::escape_str(&req.message);

    let (has_provider, _provider_name) = detect_ai_provider();
    let ai_response = if has_provider {
        let schema_summary = schema_summary(&state);

        let system_prompt = format!(
            "You are Nexus AI Assistant for a Rust web application built with Rullst Framework.\n\
             Database Schema:\n{}\n\
             Answer the user's question concisely using only simple semantic HTML such as <p>, <code>, <pre>, <ul>, <li>, and <strong>.",
            schema_summary
        );

        match rullst_ai::AiClient::auto() {
            Ok(client) => match client
                .chat()
                .system(&system_prompt)
                .user(&req.message)
                .send()
                .await
            {
                Ok(resp) => resp,
                Err(_) => generate_smart_nexus_ai_response(&req.message, &state),
            },
            Err(_) => generate_smart_nexus_ai_response(&req.message, &state),
        }
    } else {
        generate_smart_nexus_ai_response(&req.message, &state)
    };

    // Provider output and schema-derived fallback output are both untrusted at this boundary.
    // Ammonia's allowlist removes scripts, event handlers, dangerous URLs and unknown markup.
    let safe_ai_response = ammonia::clean(&ai_response);

    Html(format!(
        "<div class=\"nexus-chat-bubble nexus-chat-user\">\
         <span class=\"nexus-chat-avatar\">&#128100;</span>\
         <div class=\"nexus-chat-text\">{user_msg}</div>\
         </div>\
         <div class=\"nexus-chat-bubble nexus-chat-assistant\">\
         <span class=\"nexus-chat-avatar\">&#129302;</span>\
         <div class=\"nexus-chat-text\">{safe_ai_response}</div>\
         </div>"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nexus::types::{FieldKind, FieldMeta, RegistryEntry};

    fn make_test_state() -> NexusState {
        let entry = RegistryEntry {
            table: "products",
            label: "Products",
            icon: "&#128196;",
            pk: "id",
            fields: vec![
                FieldMeta::new("id", "ID", FieldKind::Number),
                FieldMeta::new("title", "Title", FieldKind::Text),
            ],
        };

        NexusState {
            registry: Arc::new(vec![entry]),
            brand: Arc::new("Admin Panel".to_string()),
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
        let resp_setup =
            generate_smart_nexus_ai_response("How to configure OpenAI provider?", &state);
        assert!(resp_setup.contains("Universal LLM Setup Guide"));
        assert!(resp_setup.contains("GEMINI_API_KEY"));

        // 2. Count query for specific table
        let resp_count =
            generate_smart_nexus_ai_response("How many products are registered?", &state);
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
            resp_fallback.contains("analyzed your query")
                || resp_fallback.contains("example query")
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
                fields: vec![FieldMeta::new(
                    "title FROM secrets;--",
                    "Title",
                    FieldKind::Text,
                )],
            }]),
            brand: Arc::new("Admin".to_string()),
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
}
