//! The tenant-scoped `Post` model and the landing page that lists and stores it.

use crate::showcase_nav::{render_shared_styles, render_showcase_nav};
use axum::http::StatusCode;
use axum::{Extension, Form};
use rullst::db::FromRow;
use rullst::security::TenantContext;
use rullst::{
    html,
    response::{Html, Redirect},
};
use rullst_orm::with_tenant;

// --- Post Model & Active Record Query Builder ---
/// A story owned by one tenant.
///
/// `tenant_column` makes every generated query fail closed outside
/// `with_tenant(...)` and bind the active tenant inside it; `save()` stamps the
/// active tenant. Handlers take the tenant from the membership-checked
/// [`TenantContext`] that `TenantLayer` inserts, never from the raw header.
#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "posts", tenant_column = "tenant_id")]
pub struct Post {
    pub id: i32,
    pub tenant_id: String,
    pub title: String,
    pub body: String,
}

impl rullst_nexus::NexusModel for Post {
    fn nexus_table() -> &'static str {
        "posts"
    }
    fn nexus_label() -> &'static str {
        "Blog Posts"
    }
    fn nexus_icon() -> &'static str {
        "📝"
    }
    fn nexus_pk() -> &'static str {
        "id"
    }
    fn nexus_fields() -> Vec<rullst_nexus::FieldMeta> {
        vec![
            rullst_nexus::FieldMeta {
                name: "id",
                label: "ID",
                kind: rullst_nexus::FieldKind::Number,
                hidden: true,
                readonly: true,
            },
            rullst_nexus::FieldMeta {
                name: "tenant_id",
                label: "Tenant ID",
                kind: rullst_nexus::FieldKind::Text,
                hidden: false,
                readonly: false,
            },
            rullst_nexus::FieldMeta {
                name: "title",
                label: "Title",
                kind: rullst_nexus::FieldKind::Text,
                hidden: false,
                readonly: false,
            },
            rullst_nexus::FieldMeta {
                name: "body",
                label: "Content",
                kind: rullst_nexus::FieldKind::Textarea,
                hidden: false,
                readonly: false,
            },
        ]
    }
}

/// Creates the SQLite `posts` table when it does not exist yet.
pub async fn create_schema() -> Result<(), rullst_orm::Error> {
    let pool = rullst_orm::Orm::pool()?;
    rullst::db::sqlx::query(
        "CREATE TABLE IF NOT EXISTS posts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tenant_id TEXT NOT NULL,
            title TEXT NOT NULL,
            body TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Longest accepted title, in characters.
pub const MAX_TITLE_CHARS: usize = 120;
/// Longest accepted body, in characters.
pub const MAX_BODY_CHARS: usize = 4_000;
/// Stories this public showcase keeps per tenant.
pub const MAX_POSTS_PER_TENANT: i64 = 100;
/// Newest stories rendered on the landing page.
pub const LISTED_POSTS: usize = 20;
/// Largest accepted request body. URL-encoding can triple multibyte text.
pub const MAX_FORM_BYTES: usize = 64 * 1024;

#[derive(serde::Deserialize)]
pub struct CreatePostForm {
    pub title: String,
    pub body: String,
}

/// Why a story was not stored.
#[derive(Debug)]
pub enum CreatePostError {
    /// The title or body is blank or longer than its limit.
    Invalid(&'static str),
    /// The tenant already holds [`MAX_POSTS_PER_TENANT`] stories.
    QuotaReached,
    /// The database rejected the query.
    Database(rullst_orm::Error),
}

impl From<rullst_orm::Error> for CreatePostError {
    fn from(error: rullst_orm::Error) -> Self {
        Self::Database(error)
    }
}

/// Trims and bounds a submitted story.
pub fn validate_post(title: &str, body: &str) -> Result<(String, String), CreatePostError> {
    let (title, body) = (title.trim(), body.trim());
    if title.is_empty() || body.is_empty() {
        return Err(CreatePostError::Invalid(
            "A story needs a title and a body.",
        ));
    }
    if title.chars().count() > MAX_TITLE_CHARS {
        return Err(CreatePostError::Invalid(
            "The title is limited to 120 characters.",
        ));
    }
    if body.chars().count() > MAX_BODY_CHARS {
        return Err(CreatePostError::Invalid(
            "The body is limited to 4,000 characters.",
        ));
    }
    Ok((title.to_string(), body.to_string()))
}

/// Stores a validated story in `tenant_id` unless the tenant is at its quota.
///
/// The count and the insert are separate statements, so concurrent requests
/// can overshoot the quota by at most the number of requests in flight.
pub async fn create_post(
    tenant_id: &str,
    title: &str,
    body: &str,
) -> Result<Post, CreatePostError> {
    let (title, body) = validate_post(title, body)?;
    with_tenant(tenant_id.to_string(), async {
        if Post::query().count().await? >= MAX_POSTS_PER_TENANT {
            return Err(CreatePostError::QuotaReached);
        }
        let mut post = Post {
            id: 0,
            tenant_id: tenant_id.to_string(),
            title,
            body,
        };
        post.save().await?;
        Ok(post)
    })
    .await
}

/// The tenant's newest [`LISTED_POSTS`] stories, newest first.
pub async fn recent_posts(tenant_id: &str) -> Result<Vec<Post>, rullst_orm::Error> {
    // Build the query inside the scope: `query()` binds the active tenant.
    with_tenant(tenant_id.to_string(), async {
        Post::query()
            .order_by_desc("id")
            .limit(LISTED_POSTS)
            .get()
            .await
    })
    .await
}

fn render_post_list(posts: &[Post]) -> String {
    if posts.is_empty() {
        html! {
            <div style="text-align: center; color: var(--text-muted); padding: 3rem; font-style: italic; background: #05070c; border: 1px dashed #1e293b; border-radius: 0.5rem;">
                "No published stories in this tenant context. Use the form above to publish one!"
            </div>
        }
    } else {
        let items: String = posts
            .iter()
            .map(|post| {
                html! {
                    <div style="background: #0d121f; border-left: 4px solid #3b82f6; border-radius: 0.5rem; padding: 1.5rem; margin-bottom: 1rem; border: 1px solid #1e293b; border-left-width: 4px;">
                        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.5rem;">
                            <h3 style="margin: 0; font-size: 1.25rem; color: #fff;">{&post.title}</h3>
                            <span style="font-size: 0.72rem; color: #60a5fa; background: rgba(59, 130, 246, 0.15); padding: 0.2rem 0.5rem; border-radius: 0.25rem;">
                                "Tenant: " {&post.tenant_id}
                            </span>
                        </div>
                        <p style="color: #cbd5e1; margin: 0; line-height: 1.6; font-size: 0.95rem; white-space: pre-wrap;">{&post.body}</p>
                    </div>
                }
            })
            .collect();
        items
    }
}

// --- Route Handlers ---

/// Server-rendered HTML landing page (`/`).
pub async fn index(
    Extension(csrf_token): Extension<rullst::security::CsrfToken>,
    Extension(tenant): Extension<TenantContext>,
) -> Result<Html<String>, StatusCode> {
    let posts = recent_posts(&tenant.tenant_id)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let nav = render_showcase_nav("/");
    let styles = render_shared_styles();
    let post_list_html = render_post_list(&posts);

    Ok(Html(html! {
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <title>"Rullst Sovereign SaaS Blog & Publisher"</title>
                <link rel="icon" type="image/png" href="https://raw.githubusercontent.com/Rullst/Rullst/main/Rullst.png" />
                <style>{ rullst::html::RawHtml(styles) }</style>
            </head>
            <body>
                { rullst::html::RawHtml(nav) }
                <div class="container">
                    <div class="card">
                        <div style="display: flex; justify-content: space-between; align-items: flex-start;">
                            <div>
                                <h1 class="card-title">
                                    "⚡ Typed Server-Side Rendering"
                                    <span class="feature-tag tag-orm">"rullst-core"</span>
                                </h1>
                                <p style="color: var(--text-muted); margin-bottom: 1.5rem;">
                                    "Typed declarative HTML generated by Rullst's `html!` macro and served through Axum. Optional HTMX behavior still requires its browser runtime; latency and payload size depend on the deployed application."
                                </p>
                            </div>
                        </div>

                        <form method="post" action="/posts" style="background: #05070c; border: 1px solid #1e293b; border-radius: 0.5rem; padding: 1.5rem;">
                            <input type="hidden" name="_token" value={csrf_token.as_str()} />
                            <h3 style="margin-top: 0; color: #38bdf8; font-size: 1.1rem; margin-bottom: 1rem;">"Publish a New Story (Active Record)"</h3>
                            <div style="margin-bottom: 1rem;">
                                <label style="display: block; font-size: 0.85rem; color: #94a3b8; margin-bottom: 0.4rem;">"Article Title"</label>
                                <input type="text" name="title" maxlength="120" placeholder="e.g. Memory Safety with Rust 2024" required="required" style="width: 100%; background: #0d121f; border: 1px solid #334155; border-radius: 0.375rem; padding: 0.65rem 0.85rem; color: #fff;" />
                            </div>
                            <div style="margin-bottom: 1rem;">
                                <label style="display: block; font-size: 0.85rem; color: #94a3b8; margin-bottom: 0.4rem;">"Content (Markdown/Text)"</label>
                                <textarea name="body" rows="4" maxlength="4000" placeholder="Write your post content here..." required="required" style="width: 100%; background: #0d121f; border: 1px solid #334155; border-radius: 0.375rem; padding: 0.65rem 0.85rem; color: #fff;"></textarea>
                            </div>
                            <p class="form-hint">"Titles up to 120 characters, bodies up to 4,000. Each tenant keeps at most 100 stories; the newest 20 are listed below."</p>
                            <button type="submit" class="btn">"Publish Article"</button>
                        </form>
                    </div>

                    <div class="card">
                        <h2 class="card-title">"Published Stories (Scoped by Tenant)"</h2>
                        <div>
                            { rullst::html::RawHtml(post_list_html) }
                        </div>
                    </div>
                </div>
            </body>
        </html>
    }))
}

/// Stores a new post via Active Record in the request's tenant.
pub async fn store(
    Extension(tenant): Extension<TenantContext>,
    Form(form): Form<CreatePostForm>,
) -> Result<Redirect, (StatusCode, &'static str)> {
    match create_post(&tenant.tenant_id, &form.title, &form.body).await {
        Ok(_) => Ok(Redirect::to("/")),
        Err(CreatePostError::Invalid(reason)) => Err((StatusCode::UNPROCESSABLE_ENTITY, reason)),
        Err(CreatePostError::QuotaReached) => Err((
            StatusCode::FORBIDDEN,
            "This showcase tenant already holds its 100 stories.",
        )),
        Err(CreatePostError::Database(error)) => {
            tracing::warn!(%error, "storing a showcase story failed");
            Err((
                StatusCode::SERVICE_UNAVAILABLE,
                "The showcase database is unavailable.",
            ))
        }
    }
}
