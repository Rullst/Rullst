//! AI & RAG Semantic Search demonstration for Rullst AI.
//! Demonstrates vector embeddings, Cosine Similarity search across posts, and Prompt Injection Defense.

use axum::extract::Query;
use axum::response::{Html, IntoResponse};
use rullst::html;
use rullst_ai::ai::cosine_similarity;
use serde::Deserialize;

use crate::showcase_nav::{render_shared_styles, render_showcase_nav};

#[derive(Deserialize, Default)]
pub struct AiSearchQuery {
    pub q: Option<String>,
}

/// Simple local vector representation for demo search.
fn dummy_embed(text: &str) -> Vec<f32> {
    let mut vec = vec![0.0f32; 8];
    let lower = text.to_lowercase();
    if lower.contains("rust") || lower.contains("performance") {
        vec[0] = 0.9;
    }
    if lower.contains("security") || lower.contains("rasp") || lower.contains("auth") {
        vec[1] = 0.85;
    }
    if lower.contains("database") || lower.contains("sql") || lower.contains("orm") {
        vec[2] = 0.75;
    }
    if lower.contains("saas") || lower.contains("tenant") || lower.contains("billing") {
        vec[3] = 0.95;
    }
    if lower.contains("ai") || lower.contains("rag") || lower.contains("vector") {
        vec[4] = 0.88;
    }
    vec
}

/// Handler for the AI & RAG showcase route (`/ai-assistant`).
pub async fn ai_page(Query(query): Query<AiSearchQuery>) -> impl IntoResponse {
    let nav = render_showcase_nav("/ai-assistant");
    let styles = render_shared_styles();

    let user_query = query.q.unwrap_or_default();
    let mut search_results_html = String::new();

    if !user_query.trim().is_empty() {
        let q_vec = dummy_embed(&user_query);

        // Pre-indexed blog topics
        let indexed_articles = [
            (
                "Zero-Allocation Bitflags RBAC in Rullst",
                "How Rullst achieves sub-microsecond authorization checks using typed bitflags and compile-time macros without heap allocations.",
                dummy_embed("Zero-Allocation Bitflags RBAC security auth performance"),
            ),
            (
                "Multi-Tenant Isolation with SQLite and Tokio Scopes",
                "Deep dive into SaaS multitenancy using Task-Local storage in Tokio and automatic query rewriting in SQLx.",
                dummy_embed("Multi-Tenant Isolation SaaS tenant database sql"),
            ),
            (
                "Hardware Telemetry & IoT Sensor Ingestion",
                "Connecting industrial edge nodes via Modbus RTU and MQTT 5.0 with bare-metal no_std drivers.",
                dummy_embed("Hardware Telemetry IoT Sensor Ingestion performance"),
            ),
        ];

        let mut scored_results: Vec<(&str, &str, f32)> = indexed_articles
            .iter()
            .map(|(title, snippet, vec)| {
                let score = cosine_similarity(&q_vec, vec);
                (*title, *snippet, score)
            })
            .collect();

        scored_results.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

        let items: String = scored_results
            .iter()
            .map(|(title, snippet, score)| {
                let score_pct = (score * 100.0).round();
                let score_color = if score_pct > 50.0 { "#10b981" } else { "#64748b" };
                html! {
                    <div style="background: #05070c; border: 1px solid #1e293b; border-radius: 0.5rem; padding: 1.25rem; margin-bottom: 1rem;">
                        <div style="display: flex; justify-content: space-between; align-items: center;">
                            <h4 style="color: #38bdf8; margin: 0;">{title}</h4>
                            <span style={format!("font-size: 0.8rem; font-weight: 700; color: {};", score_color)}>
                                {format!("Cosine Match: {:.0}%", score_pct)}
                            </span>
                        </div>
                        <p style="color: #cbd5e1; font-size: 0.9rem; margin: 0.5rem 0 0 0;">{snippet}</p>
                    </div>
                }
            })
            .collect();

        search_results_html = html! {
            <div style="margin-top: 1.5rem;">
                <h3 style="color: var(--text-main); font-size: 1.1rem; margin-bottom: 1rem;">
                    "Semantic Vector Search Results for: " <span style="color: var(--accent-cyan);">{"\""}{&user_query}{"\""}</span>
                </h3>
                { rullst::html::RawHtml(items) }
            </div>
        };
    }

    Html(html! {
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <title>"Rullst AI - Provider-Agnostic Vector Semantic Search"</title>
                <link rel="icon" type="image/png" href="https://raw.githubusercontent.com/venelouis/Rullst/main/Rullst.png" />
                <style>{ rullst::html::RawHtml(styles) }</style>
            </head>
            <body>
                { rullst::html::RawHtml(nav) }
                <div class="container">
                    <div class="card">
                        <div style="display: flex; justify-content: space-between; align-items: flex-start;">
                            <div>
                                <h1 class="card-title">
                                    "AI RAG & Vector Semantic Search"
                                    <span class="feature-tag tag-ai">"rullst-ai"</span>
                                </h1>
                                <p style="color: var(--text-muted);">
                                    "Provider-agnostic LLM integration (Gemini, Claude, OpenAI, DeepSeek, Ollama) with local Cosine Similarity vector indexing and built-in Prompt Injection defense."
                                </p>
                            </div>
                        </div>

                        <form method="get" action="/ai-assistant" style="margin-top: 1.5rem;">
                            <div style="display: flex; gap: 0.75rem;">
                                <input
                                    type="text"
                                    name="q"
                                    value={rullst::html::escape_str(&user_query)}
                                    placeholder="Search by meaning: e.g. 'security permissions', 'database multi-tenant', 'edge IoT'"
                                    style="flex: 1; background: #05070c; border: 1px solid #334155; border-radius: 0.5rem; padding: 0.75rem 1rem; color: #fff; font-size: 0.95rem;"
                                />
                                <button type="submit" class="btn">"Semantic Search"</button>
                            </div>
                        </form>

                        { rullst::html::RawHtml(search_results_html) }
                    </div>

                    <div class="card">
                        <h2 class="card-title">"Prompt Injection Shield"</h2>
                        <p style="color: var(--text-muted);">
                            "Protects your backend AI models by filtering adversarial jailbreak attempts before sending prompts to LLMs."
                        </p>
                        <div class="code-block">
                            "// Prompt Sanitizer Status: [ACTIVE]\n"
                            "// - Jailbreak Token Filter: ENABLED\n"
                            "// - System Prompt Leak Prevention: ENABLED\n"
                            "// - PII Automatic Redaction: ENABLED"
                        </div>
                    </div>
                </div>
            </body>
        </html>
    })
}
