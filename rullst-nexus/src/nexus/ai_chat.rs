use axum::{extract::State, response::Html};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::nexus::crud::sanitize_identifier;
use crate::nexus::types::NexusState;
#[cfg(feature = "ai")]
use crate::nexus::{
    crud::field_kind_label,
    ui::{render_shell, render_sidebar, wants_fragment},
};

// Compiled in every build, so its tests also run with `ai` (the all-feature
// mutation gate); only a build without `ai` mounts it.
pub(crate) mod disabled;
#[cfg(feature = "ai")]
mod sanitize;
#[cfg(feature = "ai")]
use sanitize::sanitize_chat_html;

/// Whether this build serves the AI assistant (the `ai` feature). Without it
/// Nexus does not link `rullst-ai`, hides the assistant link, and the router
/// answers `/chat` and `/chat/query` with 404 and a notice naming the feature.
pub(crate) const AI_ASSISTANT: bool = cfg!(feature = "ai");

#[derive(Deserialize, Serialize)]
pub struct ChatRequest {
    pub message: String,
}

#[cfg(feature = "ai")]
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

/// Whether an AI provider is configured, and its name or the reason none is.
/// Without the `ai` feature this reports that the assistant is not enabled.
pub fn detect_ai_provider() -> (bool, String) {
    #[cfg(feature = "ai")]
    {
        match rullst_ai::AutoAiConfig::from_env() {
            Ok(config) if !config.provider_names().is_empty() => {
                (true, config.provider_names().join(" → "))
            }
            Ok(_) => (false, "Offline Embedded Intelligence".into()),
            Err(_) => (false, "AI configuration incomplete or invalid".into()),
        }
    }
    #[cfg(not(feature = "ai"))]
    {
        (false, "AI assistant not enabled in this build".into())
    }
}

// The assistant's reply builder behind `nexus_chat_query`.
#[doc(hidden)]
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
             <p>Rullst AI supports <strong>the configured providers below</strong>. Add the corresponding variables to your project's <code class=\"nexus-code\">.env</code> file:</p>\
             <ul style=\"margin: 0.5rem 0; padding-left: 1.25rem; font-size: 0.85rem;\">\
               <li><strong>Google Gemini:</strong> <code class=\"nexus-code\">GEMINI_API_KEY=your_key</code></li>\
               <li><strong>OpenAI / ChatGPT:</strong> <code class=\"nexus-code\">OPENAI_API_KEY=your_key</code></li>\
               <li><strong>Anthropic Claude:</strong> <code class=\"nexus-code\">ANTHROPIC_API_KEY=your_key</code></li>\
               <li><strong>Local Ollama:</strong> <code class=\"nexus-code\">OLLAMA_HOST=http://localhost:11434</code></li>\
               <li><strong>DeepSeek:</strong> <code class=\"nexus-code\">DEEPSEEK_API_KEY=your_key</code></li>\
               <li><strong>Groq:</strong> <code class=\"nexus-code\">GROQ_API_KEY=your_key; GROQ_MODEL=your_model</code></li>\
               <li><strong>Custom compatible endpoint:</strong> <code class=\"nexus-code\">OPENAI_BASE_URL=https://...; OPENAI_API_KEY=your_key; OPENAI_MODEL=your_model</code></li>\
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
///
/// Without the `ai` feature it renders the not-enabled notice instead; the
/// Nexus router then answers it with 404.
pub async fn nexus_chat_page(
    State(state): State<Arc<NexusState>>,
    headers: axum::http::HeaderMap,
) -> Html<String> {
    #[cfg(not(feature = "ai"))]
    {
        Html(disabled::page_html(&state, &headers))
    }
    #[cfg(feature = "ai")]
    {
        chat_page_html(&state, &headers)
    }
}

#[cfg(feature = "ai")]
fn chat_page_html(state: &NexusState, headers: &axum::http::HeaderMap) -> Html<String> {
    let schema_summary = schema_summary(state);

    let mut content = String::new();
    content.push_str("<div class=\"nexus-page-header\">");
    content.push_str("<h1 class=\"nexus-page-title\">&#129302; AI Query Assistant</h1>");
    content.push_str("<p class=\"nexus-page-subtitle\">Ask questions about your data in plain language. Powered by <strong>rullst::ai</strong>.</p>");
    content.push_str("</div>");
    content.push_str("<div class=\"nexus-chat-layout\">");

    // Left Control Panel: Schema & Quick Commands
    content.push_str("<div class=\"nexus-chat-schema\">");

    // Quick Preset Commands
    content.push_str("<div class=\"nexus-card nexus-chat-card\">");
    content.push_str("<div class=\"nexus-schema-title\">&#9889; Quick Commands</div>");
    content.push_str("<div class=\"nexus-quick-commands\">");
    for (prompt, label) in [
        (
            "Show all registered database tables",
            "&#128202; Show All Tables",
        ),
        (
            "How many records exist in the database?",
            "&#128200; Count Table Rows",
        ),
        (
            "How do I configure an AI provider key in .env?",
            "&#127760; LLM Setup Instructions",
        ),
    ] {
        content.push_str(&format!(
            "<button type=\"button\" class=\"nexus-btn nexus-quick-command\" data-nexus-prompt=\"{prompt}\">{label}</button>"
        ));
    }
    content.push_str("</div></div>");

    // LLM Provider Setup Banner
    content.push_str("<div class=\"nexus-card nexus-chat-card nexus-chat-banner\">");
    content
        .push_str("<div class=\"nexus-chat-banner-title\">&#127760; Universal LLM Support</div>");
    content.push_str("<p class=\"nexus-chat-banner-text\">Connect to <strong>Gemini, OpenAI, Claude, Ollama, DeepSeek, Qwen, or Kimi</strong> via your <code class=\"nexus-code\">.env</code> file:</p>");
    content.push_str("<pre class=\"nexus-schema-pre nexus-schema-pre-compact\">GEMINI_API_KEY=key\nOPENAI_API_KEY=key\nANTHROPIC_API_KEY=key\nDEEPSEEK_API_KEY=key\nGROQ_API_KEY=key\nGROQ_MODEL=model\nOLLAMA_HOST=http://...\nOPENAI_BASE_URL=https://...\nOPENAI_MODEL=model</pre>");
    content.push_str("</div>");

    // Database Schema Summary
    content.push_str("<div class=\"nexus-card nexus-chat-card\">");
    content.push_str("<div class=\"nexus-schema-title\">&#128202; Database Schema</div>");
    content.push_str("<pre class=\"nexus-schema-pre\">");
    content.push_str(&rullst_core::html::escape_str(&schema_summary));
    content.push_str("</pre></div></div>");

    // Right Panel: Chat Messages
    content.push_str("<div class=\"nexus-chat-panel\">");
    content.push_str("<div class=\"nexus-chat-messages\" id=\"nexus-chat-messages\">");
    content.push_str("<div class=\"nexus-chat-bubble nexus-chat-assistant\">");
    content.push_str("<span class=\"nexus-chat-avatar\">&#129302;</span>");
    content.push_str("<div class=\"nexus-chat-text\">Hello! I have full offline intelligence about your database schema. Ask me anything &mdash; for example:<br><em>\"List all courses\"</em>, <em>\"Show tables\"</em>, or <em>\"How do I setup DeepSeek/Qwen?\"</em><br><br><small class=\"nexus-chat-tip\">&#128161; <b>Tip:</b> Click any Quick Command on the left or type your query below.</small></div>");
    content.push_str("</div></div>");
    // nexus.js resets the form and scrolls the transcript after each answer.
    content.push_str("<form class=\"nexus-chat-form\" hx-post=\"/nexus/chat/query\" hx-target=\"#nexus-chat-messages\" hx-swap=\"beforeend\">");
    content.push_str("<input type=\"text\" name=\"message\" id=\"nexus-chat-input\" class=\"nexus-chat-input\" placeholder=\"Ask about your data...\" aria-label=\"Ask the AI assistant\" autocomplete=\"off\" required />");
    content.push_str(
        "<button type=\"submit\" class=\"nexus-btn nexus-btn-ai\">Send &#9992;&#65039;</button>",
    );
    content.push_str("</form></div></div>");

    if wants_fragment(headers) {
        Html(content)
    } else {
        Html(render_shell(state, &render_sidebar(state, None), &content))
    }
}

/// POST /nexus/chat/query — AI Assistant HTMX query endpoint.
///
/// Without the `ai` feature it answers the not-enabled notice instead; the
/// Nexus router then answers it with 404.
pub async fn nexus_chat_query(
    State(state): State<Arc<NexusState>>,
    axum::extract::Form(req): axum::extract::Form<ChatRequest>,
) -> Html<String> {
    #[cfg(not(feature = "ai"))]
    {
        let _ = (state, req);
        Html(disabled::query_html())
    }
    #[cfg(feature = "ai")]
    {
        chat_query_html(&state, &req.message).await
    }
}

#[cfg(feature = "ai")]
async fn chat_query_html(state: &NexusState, message: &str) -> Html<String> {
    let user_msg = rullst_core::html::escape_str(message);

    let ai_response = match provider_answer(message, state).await {
        Some(answer) => answer,
        None => generate_smart_nexus_ai_response(message, state),
    };

    // Provider output and schema-derived fallback output are both untrusted at this boundary.
    // The allowlist removes scripts, event handlers, dangerous URLs, unknown markup and every
    // element that would load a resource (such as an image beacon) by itself.
    let safe_ai_response = sanitize_chat_html(&ai_response);

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

/// The configured provider's answer: `None` without a provider or when the
/// provider fails, so the offline schema assistant answers instead.
#[cfg(feature = "ai")]
async fn provider_answer(message: &str, state: &NexusState) -> Option<String> {
    let config = rullst_ai::AutoAiConfig::from_env()
        .ok()
        .filter(|config| !config.provider_names().is_empty())?;
    let system_prompt = format!(
        "You are Nexus AI Assistant for a Rust web application built with Rullst Framework.\n\
         Database Schema:\n{}\n\
         Answer the user's question concisely using only simple semantic HTML such as <p>, <code>, <pre>, <ul>, <li>, and <strong>.",
        schema_summary(state)
    );
    config
        .into_client()
        .chat()
        .system(&system_prompt)
        .user(message)
        .send()
        .await
        .ok()
}

#[cfg(all(test, feature = "ai"))]
mod tests;
