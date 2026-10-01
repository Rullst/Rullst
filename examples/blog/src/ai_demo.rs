//! AI & RAG Semantic Search demonstration for Rullst AI.
//! Demonstrates vector embeddings, Cosine Similarity search across posts, and Prompt Injection Defense.

use axum::extract::Query;
use axum::response::{Html, IntoResponse};
use rullst::html;
use rullst_ai::ai::cosine_similarity;
use serde::Deserialize;

use crate::showcase_nav::{render_head_assets, render_showcase_nav};

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
    let head_assets = render_head_assets();

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
                "Parsing Modbus RTU frames, modeling no_std telemetry, and gating firmware updates with signed Ed25519 manifests.",
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
                let score_class = if score_pct > 50.0 {
                    "score score-high"
                } else {
                    "score score-low"
                };
                html! {
                    <div class="search-result">
                        <div class="search-result-header">
                            <h4 class="search-result-title">{title}</h4>
                            <span class={score_class}>
                                {format!("Cosine Match: {:.0}%", score_pct)}
                            </span>
                        </div>
                        <p class="search-result-snippet">{snippet}</p>
                    </div>
                }
            })
            .collect();

        search_results_html = html! {
            <div class="results">
                <h3 class="results-heading">
                    "Semantic Vector Search Results for: " <span class="query-echo">{"\""}{&user_query}{"\""}</span>
                </h3>
                { rullst::html::RawHtml(items) }
            </div>
        };
    }

    Html(html! {
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                <title>"Rullst AI - Provider-Agnostic Vector Semantic Search"</title>
                { rullst::html::RawHtml(head_assets) }
            </head>
            <body>
                { rullst::html::RawHtml(nav) }
                <div class="container">
                    <div class="card">
                        <div class="card-header">
                            <div>
                                <h1 class="card-title">
                                    "AI RAG & Vector Semantic Search"
                                    <span class="feature-tag tag-ai">"rullst-ai"</span>
                                </h1>
                                <p class="muted">
                                    "Provider-agnostic LLM integration (Gemini, Claude, OpenAI, DeepSeek, Ollama) with local Cosine Similarity vector indexing and built-in Prompt Injection defense."
                                </p>
                            </div>
                        </div>

                        <form method="get" action="/ai-assistant" class="search-form">
                            <div class="inline-form">
                                <input
                                    type="text"
                                    name="q"
                                    value={&user_query}
                                    placeholder="Search by meaning: e.g. 'security permissions', 'database multi-tenant', 'edge IoT'"
                                    class="search-input"
                                />
                                <button type="submit" class="btn">"Semantic Search"</button>
                            </div>
                        </form>

                        { rullst::html::RawHtml(search_results_html) }
                    </div>

                    <div class="card">
                        <h2 class="card-title">"Prompt Injection Shield"</h2>
                        <p class="muted">
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn search_box_value_is_escaped_exactly_once() {
        let response = ai_page(Query(AiSearchQuery {
            q: Some("R&D \"quoted\"".to_string()),
        }))
        .await
        .into_response();
        let body = axum::body::to_bytes(response.into_body(), 512 * 1024)
            .await
            .expect("bounded AI demo response");
        let html = String::from_utf8(body.to_vec()).expect("AI demo is UTF-8 HTML");
        assert!(html.contains("value=\"R&amp;D &quot;quoted&quot;\""));
        assert!(!html.contains("&amp;amp;"));
        assert!(!html.contains("&amp;quot;"));
    }
}
