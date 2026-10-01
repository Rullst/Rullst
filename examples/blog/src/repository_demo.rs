//! Data Mapper & Repository Pattern demonstration for Rullst ORM.
//! Shows decoupling between database schemas and domain aggregation models.

use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::Html;
use rullst::html;
use rullst::security::TenantContext;
use rullst_orm::Orm;
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::showcase_nav::{render_head_assets, render_showcase_nav};

/// Domain entity representing author publishing metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorAnalytics {
    pub author_name: String,
    pub total_posts: i64,
    pub total_words: i64,
    pub avg_reading_time_mins: f64,
}

/// Domain entity representing a raw database record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawPostRecord {
    pub id: i64,
    pub tenant_id: String,
    pub title: String,
    pub body: String,
}

/// Rows listed by the repository page.
pub const REPOSITORY_ROWS: i64 = 20;
/// Characters of each body included in the repository listing.
pub const BODY_PREVIEW_CHARS: i64 = 160;

/// Repository responsible for aggregations and raw read models.
///
/// These hand-written queries bypass the model's `tenant_column` scope, so
/// every one binds the tenant from the request's [`TenantContext`] itself.
pub struct PostRepository;

impl PostRepository {
    /// Aggregates the tenant's publishing metrics via a parameterized SQLx query.
    pub async fn get_tenant_analytics(
        tenant_id: &str,
    ) -> Result<Vec<AuthorAnalytics>, rullst_orm::Error> {
        let pool = Orm::pool()?;
        let rows = sqlx::query(
            "SELECT tenant_id AS author_name,
                COUNT(*) AS total_posts,
                SUM(LENGTH(body)) AS total_bytes
            FROM posts
            WHERE tenant_id = ?
            GROUP BY tenant_id",
        )
        .bind(tenant_id)
        .fetch_all(pool)
        .await?;

        let analytics = rows
            .into_iter()
            .map(|row| -> Result<AuthorAnalytics, sqlx::Error> {
                let author_name: String = row.try_get("author_name")?;
                let total_posts: i64 = row.try_get("total_posts")?;
                let total_bytes = row.try_get::<Option<i64>, _>("total_bytes")?.unwrap_or(0);
                let words = total_bytes / 5;
                let reading_time = (words as f64) / 200.0;
                Ok(AuthorAnalytics {
                    author_name,
                    total_posts,
                    total_words: words,
                    avg_reading_time_mins: (reading_time * 10.0).round() / 10.0,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(analytics)
    }

    /// Fetches the tenant's newest [`REPOSITORY_ROWS`] posts, with a bounded
    /// body preview, directly via a Data Mapper SQLx query.
    pub async fn get_tenant_posts(
        tenant_id: &str,
    ) -> Result<Vec<RawPostRecord>, rullst_orm::Error> {
        let pool = Orm::pool()?;
        let rows = sqlx::query(
            "SELECT id, tenant_id, title, substr(body, 1, ?) AS body FROM posts
            WHERE tenant_id = ? ORDER BY id DESC LIMIT ?",
        )
        .bind(BODY_PREVIEW_CHARS)
        .bind(tenant_id)
        .bind(REPOSITORY_ROWS)
        .fetch_all(pool)
        .await?;

        let posts = rows
            .into_iter()
            .map(|row| -> Result<RawPostRecord, sqlx::Error> {
                let id = match row.try_get::<i64, _>("id") {
                    Ok(id) => id,
                    Err(i64_error) => match row.try_get::<i32, _>("id") {
                        Ok(id) => i64::from(id),
                        Err(_) => return Err(i64_error),
                    },
                };
                Ok(RawPostRecord {
                    id,
                    tenant_id: row.try_get("tenant_id")?,
                    title: row.try_get("title")?,
                    body: row.try_get("body")?,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(posts)
    }
}

/// Handler for the Repository ORM showcase route (`/posts/repository`).
pub async fn repository_page(
    Extension(tenant): Extension<TenantContext>,
) -> Result<Html<String>, StatusCode> {
    let nav = render_showcase_nav("/posts/repository");
    let head_assets = render_head_assets();

    let analytics = PostRepository::get_tenant_analytics(&tenant.tenant_id)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    let all_posts = PostRepository::get_tenant_posts(&tenant.tenant_id)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    let rows_html: String = analytics
        .iter()
        .map(|a| {
            html! {
                <tr>
                    <td class="cell-strong">{&a.author_name}</td>
                    <td class="numeric">{a.total_posts}</td>
                    <td class="numeric">{a.total_words}</td>
                    <td class="numeric cell-good">{format!("{:.1} min", a.avg_reading_time_mins)}</td>
                </tr>
            }
        })
        .collect();

    let post_rows_html: String = all_posts
        .iter()
        .map(|p| {
            html! {
                <tr>
                    <td class="cell-id">{format!("#{}", p.id)}</td>
                    <td><span class="tenant-chip">{&p.tenant_id}</span></td>
                    <td class="cell-title">{&p.title}</td>
                    <td class="cell-preview">{&p.body}</td>
                </tr>
            }
        })
        .collect();

    Ok(Html(html! {
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                <title>"Rullst ORM - Repository & Data Mapper Pattern"</title>
                { rullst::html::RawHtml(head_assets) }
            </head>
            <body>
                { rullst::html::RawHtml(nav) }
                <div class="container">
                    <div class="card">
                        <div class="card-header">
                            <div>
                                <h1 class="card-title">
                                    "Data Mapper & Repository Pattern"
                                    <span class="feature-tag tag-orm">"rullst-orm"</span>
                                </h1>
                                <p class="lead">
                                    "While Active Record handles high-velocity CRUD, Rullst's Repository pattern provides clean separation of concerns for domain aggregations, CQRS read models, and cross-table analytics."
                                </p>
                            </div>
                        </div>

                        <div class="code-block spaced">
                            "// Rust Implementation in repository_demo.rs:\n"
                            "let analytics = PostRepository::get_tenant_analytics(&amp;tenant.tenant_id).await?;\n"
                            "let posts = PostRepository::get_tenant_posts(&amp;tenant.tenant_id).await?;\n"
                            "// -> Raw SQL bypasses the model's tenant scope, so each query binds the request's TenantContext."
                        </div>

                        <h3 class="section-heading">"Domain Analytics for the Active Tenant"</h3>
                        <table class="data-table spaced">
                            <thead>
                                <tr>
                                    <th>"Author / Tenant"</th>
                                    <th class="numeric">"Total Published Posts"</th>
                                    <th class="numeric">"Estimated Words"</th>
                                    <th class="numeric">"Est. Reading Time"</th>
                                </tr>
                            </thead>
                            <tbody>
                                { rullst::html::RawHtml(rows_html) }
                            </tbody>
                        </table>

                        <h3 class="section-heading">"Newest Tenant Records (`posts` Table, 20 rows, 160-character previews)"</h3>
                        <table class="data-table dense">
                            <thead>
                                <tr>
                                    <th>"ID"</th>
                                    <th>"Tenant"</th>
                                    <th>"Title"</th>
                                    <th>"Body Preview"</th>
                                </tr>
                            </thead>
                            <tbody>
                                { rullst::html::RawHtml(post_rows_html) }
                            </tbody>
                        </table>
                    </div>

                    <div class="card">
                        <h2 class="card-title">"Explicit Indexing & Query Review"</h2>
                        <p class="muted">
                            "Repository queries remain ordinary parameterized SQLx. Add indexes through reviewed migrations and inspect the real query plan for each supported database. Rullst does not infer index migrations from doc comments."
                        </p>
                        <div class="code-block">
                            "CREATE INDEX idx_posts_tenant_id_title\n"
                            "    ON posts (tenant_id, title);\n"
                            "\n"
                            "// Verify with EXPLAIN on the exact deployed backend."
                        </div>
                    </div>
                </div>
            </body>
        </html>
    }))
}
