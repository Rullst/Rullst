//! Data Mapper & Repository Pattern demonstration for Rullst ORM.
//! Shows decoupling between database schemas and domain aggregation models.

use axum::response::{Html, IntoResponse};
use rullst::html;
use rullst_orm::Orm;
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::showcase_nav::{render_shared_styles, render_showcase_nav};

/// Domain entity representing author publishing metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorAnalytics {
    pub author_name: String,
    pub total_posts: i64,
    pub total_words: i64,
    pub avg_reading_time_mins: f64,
}

/// Repository responsible for complex aggregations and cross-table mappings.
pub struct PostRepository;

impl PostRepository {
    /// Aggregates author publishing metrics via parameterized SQLx query.
    pub async fn get_author_analytics() -> Result<Vec<AuthorAnalytics>, sqlx::Error> {
        let pool = Orm::pool();
        let rows = sqlx::query(
            "SELECT 
                COALESCE(tenant_id, 'community') as author_name,
                COUNT(*) as total_posts,
                SUM(LENGTH(body)) as total_bytes
            FROM posts
            GROUP BY tenant_id
            ORDER BY total_posts DESC"
        )
        .fetch_all(pool)
        .await?;

        let analytics = rows
            .into_iter()
            .map(|r| {
                let author_name: String = r.get("author_name");
                let total_posts: i64 = r.get("total_posts");
                let total_bytes: i64 = r.try_get("total_bytes").unwrap_or(0);
                let words = total_bytes / 5;
                let reading_time = (words as f64) / 200.0;
                AuthorAnalytics {
                    author_name,
                    total_posts,
                    total_words: words,
                    avg_reading_time_mins: (reading_time * 10.0).round() / 10.0,
                }
            })
            .collect();

        Ok(analytics)
    }
}

/// Handler for the Repository ORM showcase route (`/posts/repository`).
pub async fn repository_page() -> impl IntoResponse {
    let nav = render_showcase_nav("/posts/repository");
    let styles = render_shared_styles();

    let analytics = PostRepository::get_author_analytics()
        .await
        .unwrap_or_default();

    let rows_html: String = analytics
        .iter()
        .map(|a| {
            html! {
                <tr style="border-bottom: 1px solid #1e293b;">
                    <td style="padding: 1rem; font-weight: 600; color: #38bdf8;">{&a.author_name}</td>
                    <td style="padding: 1rem; text-align: center;">{a.total_posts}</td>
                    <td style="padding: 1rem; text-align: center;">{a.total_words}</td>
                    <td style="padding: 1rem; text-align: center; color: #10b981;">{format!("{:.1} min", a.avg_reading_time_mins)}</td>
                </tr>
            }
        })
        .collect();

    Html(html! {
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <title>"Rullst ORM - Repository & Data Mapper Pattern"</title>
                <style>{ rullst::html::RawHtml(styles) }</style>
            </head>
            <body>
                { rullst::html::RawHtml(nav) }
                <div class="container">
                    <div class="card">
                        <div style="display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 1rem;">
                            <div>
                                <h1 class="card-title">
                                    "Data Mapper & Repository Pattern"
                                    <span class="feature-tag tag-orm">"rullst-orm"</span>
                                </h1>
                                <p style="color: var(--text-muted); margin: 0;">
                                    "While Active Record handles high-velocity CRUD, Rullst's Repository pattern provides clean separation of concerns for domain aggregations, CQRS read models, and cross-table analytics."
                                </p>
                            </div>
                        </div>

                        <div class="code-block" style="margin-bottom: 1.5rem;">
                            "// Rust Implementation in repository_demo.rs:\n"
                            "let analytics = PostRepository::get_author_analytics().await?;\n"
                            "// -> Aggregates multi-tenant records without exposing underlying SQLx connection pool directly."
                        </div>

                        <table style="width: 100%; border-collapse: collapse; text-align: left; background: #05070c; border-radius: 0.5rem; overflow: hidden; border: 1px solid #1e293b;">
                            <thead>
                                <tr style="background: rgba(30, 41, 59, 0.8); border-bottom: 2px solid #334155; color: #94a3b8; font-size: 0.85rem; text-transform: uppercase;">
                                    <th style="padding: 1rem;">"Author / Tenant"</th>
                                    <th style="padding: 1rem; text-align: center;">"Total Published Posts"</th>
                                    <th style="padding: 1rem; text-align: center;">"Estimated Words"</th>
                                    <th style="padding: 1rem; text-align: center;">"Est. Reading Time"</th>
                                </tr>
                            </thead>
                            <tbody>
                                { rullst::html::RawHtml(rows_html) }
                            </tbody>
                        </table>
                    </div>

                    <div class="card">
                        <h2 class="card-title">"Intent-Based Indexing (@index)"</h2>
                        <p style="color: var(--text-muted);">
                            "Rullst ORM analyzes entity doc-comments to automatically manage index migrations and optimize query execution plans."
                        </p>
                        <div class="code-block">
                            "/// @index(tenant_id, title)\n"
                            "/// @index(created_at, desc)\n"
                            "pub struct Post { ... }\n"
                            "\n"
                            "// Index Status: [ACTIVE] idx_posts_tenant_id_title on SQLite & Postgres."
                        </div>
                    </div>
                </div>
            </body>
        </html>
    })
}
