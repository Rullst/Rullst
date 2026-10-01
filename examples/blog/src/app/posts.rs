//! The tenant-scoped `Post` model and the landing page that lists and stores it.

use crate::showcase_nav::{render_shared_styles, render_showcase_nav};
use axum::{Extension, Form};
use rullst::db::FromRow;
use rullst::{
    html,
    response::{Html, IntoResponse, Redirect},
};

// --- Post Model & Active Record Query Builder ---
#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "posts", global_scope = "apply_tenant_scope")]
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

impl PostQueryBuilder {
    pub fn apply_tenant_scope(self) -> Self {
        if let Some(tid) = rullst::multitenant::current_tenant_id() {
            self.where_eq("tenant_id", tid)
        } else {
            self
        }
    }
}

#[derive(serde::Deserialize)]
pub struct CreatePostForm {
    pub title: String,
    pub body: String,
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
            .rev()
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
) -> impl IntoResponse {
    let posts = Post::all().await.unwrap_or_default();
    let nav = render_showcase_nav("/");
    let styles = render_shared_styles();
    let post_list_html = render_post_list(&posts);

    Html(html! {
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
                                <input type="text" name="title" placeholder="e.g. Memory Safety with Rust 2024" required="required" style="width: 100%; background: #0d121f; border: 1px solid #334155; border-radius: 0.375rem; padding: 0.65rem 0.85rem; color: #fff;" />
                            </div>
                            <div style="margin-bottom: 1rem;">
                                <label style="display: block; font-size: 0.85rem; color: #94a3b8; margin-bottom: 0.4rem;">"Content (Markdown/Text)"</label>
                                <textarea name="body" rows="4" placeholder="Write your post content here..." required="required" style="width: 100%; background: #0d121f; border: 1px solid #334155; border-radius: 0.375rem; padding: 0.65rem 0.85rem; color: #fff;"></textarea>
                            </div>
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
    })
}

/// Stores a new post via Active Record
pub async fn store(Form(form): Form<CreatePostForm>) -> Redirect {
    if !form.title.trim().is_empty() && !form.body.trim().is_empty() {
        let mut post = Post {
            id: 0,
            tenant_id: rullst::multitenant::current_tenant_id()
                .unwrap_or_else(|| "community".to_string()),
            title: form.title,
            body: form.body,
        };
        let _ = post.save().await;
    }
    Redirect::to("/")
}
