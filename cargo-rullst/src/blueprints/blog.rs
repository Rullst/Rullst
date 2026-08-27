// src/blueprints/blog.rs — Blog / Content System blueprint templates.

use super::common;

pub fn file_manifest(
    project_name_safe: &str,
    hot_reload: bool,
    orm_pattern: &str,
    frontend_engine: &str,
) -> Vec<(&'static str, String)> {
    let mut manifest = Vec::new();

    let repo_decl = common::repo_mod_decl(orm_pattern);
    let is_repo = common::is_repo_mode(orm_pattern);
    let _ = project_name_safe;

    // 1. src/main.rs
    if hot_reload {
        let lib_rs = format!(
            r##"use rullst::{{routes, Router}};

pub mod migrations;
pub mod models;
{repo_decl}pub mod controllers;
pub mod pages;

pub fn router() -> Result<Router, Box<dyn std::error::Error>> {{
    let nexus_auth = rullst::nexus::NexusAuthPolicy::local_development_or_basic_from_env()?;
    let nexus = rullst::nexus::Nexus::new()
        .with_auth_policy(nexus_auth)
        .with_brand("Blog Admin")
        .register::<models::post::Post>()
        .try_build()?;

    Ok(routes![
        get("/" => controllers::blog_controller::index),
        // rullst-access: public — published blog posts are intentionally public resources.
        get("/posts/{{slug}}" => controllers::blog_controller::show),
        get("/robots.txt" => controllers::blog_controller::robots_txt),
        get("/sitemap.xml" => controllers::blog_controller::sitemap_xml),
    ].nest_axum("/nexus", nexus))
}}

#[unsafe(no_mangle)]
pub extern "C" fn rullst_router_init() -> *mut Router {{
    let router = match router() {{
        Ok(router) => router,
        Err(error) => {{
            eprintln!("Nexus startup configuration error: {{error}}");
            Router::new()
        }}
    }};
    Box::into_raw(Box::new(router))
}}
"##,
            repo_decl = repo_decl
        );
        manifest.push(("src/lib.rs", lib_rs));

        let main_rs = format!(
            r##"pub mod migrations;
pub mod models;
{repo_decl}pub mod controllers;
pub mod pages;

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    rullst::artisan!(crate::migrations::get_migrations());

    #[cfg(debug_assertions)]
    {{
        rullst::runtime::spawn(async {{
            if let Err(error) = rullst::studio::run_studio(5555).await {{
                eprintln!("Rullst Studio could not start: {{error}}");
            }}
        }});
        println!("📊 Rullst Studio running on http://127.0.0.1:5555");
    }}
    println!("🚀 Blog server starting on port 3000...");
    let is_hot = std::env::var("HOT_RELOAD").is_ok();

    let server = if is_hot {{
        let lib_path = if cfg!(target_os = "windows") {{
            format!("target/debug/{{}}", "{project_name_safe}")
        }} else {{
            format!("target/debug/lib{{}}", "{project_name_safe}")
        }};
        rullst::Server::new_hot(&lib_path)
    }} else {{
        let router = {project_name_safe}::router()?;
        rullst::Server::new(router)
    }};

    server.run(3000).await?;

    Ok(())
}}
"##,
            repo_decl = repo_decl,
            project_name_safe = project_name_safe
        );
        manifest.push(("src/main.rs", main_rs));
    } else {
        let main_rs = format!(
            r##"use rullst::{{routes, Server}};

pub mod migrations;
pub mod models;
{repo_decl}pub mod controllers;
pub mod pages;

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    // Run migrations on startup
    rullst::artisan!(crate::migrations::get_migrations());

    let nexus_auth = rullst::nexus::NexusAuthPolicy::local_development_or_basic_from_env()?;
    let nexus = rullst::nexus::Nexus::new()
        .with_auth_policy(nexus_auth)
        .with_brand("Blog Admin")
        .register::<models::post::Post>()
        .try_build()?;

    let router = routes![
        get("/" => controllers::blog_controller::index),
        // rullst-access: public — published blog posts are intentionally public resources.
        get("/posts/{{slug}}" => controllers::blog_controller::show),
        get("/robots.txt" => controllers::blog_controller::robots_txt),
        get("/sitemap.xml" => controllers::blog_controller::sitemap_xml),
    ].nest_axum("/nexus", nexus);

    #[cfg(debug_assertions)]
    {{
        rullst::runtime::spawn(async {{
            if let Err(error) = rullst::studio::run_studio(5555).await {{
                eprintln!("Rullst Studio could not start: {{error}}");
            }}
        }});
        println!("📊 Rullst Studio running on http://127.0.0.1:5555");
    }}
    println!("🚀 Blog server starting on port 3000...");
    Server::new(router)
        .run(3000)
        .await?;

    Ok(())
}}
"##,
            repo_decl = repo_decl
        );
        manifest.push(("src/main.rs", main_rs));
    }

    // 2. Migration
    let migration = r##"use rullst::db::schema::{Schema, Migration};
use rullst::db::async_trait;

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str {
        "m20260601000000_create_posts_table"
    }

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::create("posts", |table| {
            table.id();
            table.string("title").not_null();
            table.string("slug").not_null();
            table.string("content").not_null();
            table.timestamps();
        }).await?;

        // Seed initial blog posts
        let pool = rullst::db::Orm::pool()?;
        rullst::db::sqlx::query(
            "INSERT INTO posts (id, title, slug, content, created_at, updated_at) VALUES 
             (1, 'Announcing Rullst: The Ultimate Rust Framework', 'announcing-rullst', 'We are thrilled to announce Rullst, a new full-stack framework combining Axum, HTMX, and SQLite/Postgres for lightning-fast applications.', datetime('now'), datetime('now')),
             (2, 'The Power of WebAssembly Islands', 'power-of-wasm-islands', 'Wasm Islands give you the speed of server-side HTML combined with high-fidelity Wasm interactivity when needed.', datetime('now'), datetime('now'))"
        ).execute(pool).await?;

        Ok(())
    }

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::drop_if_exists("posts").await
    }
}
"##;
    manifest.push((
        "src/migrations/m20260601000000_create_posts_table.rs",
        migration.to_string(),
    ));

    let migrations_mod = r##"// Generated by Rullst.
pub mod m20260601000000_create_posts_table;

pub fn get_migrations() -> Vec<Box<dyn rullst::db::schema::Migration>> {
    vec![
        Box::new(m20260601000000_create_posts_table::MigrationImpl),
    ]
}
"##;
    manifest.push(("src/migrations/mod.rs", migrations_mod.to_string()));

    // 3. Model
    let post_model = r##"use rullst::db::{Orm, FromRow};
use rullst::nexus::{NexusModel, FieldMeta, FieldKind};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "posts")]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub slug: String,
    pub content: String,
}

impl NexusModel for Post {
    fn nexus_table() -> &'static str { "posts" }
    fn nexus_label() -> &'static str { "Posts" }
    fn nexus_icon() -> &'static str { "📝" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "title", label: "Title", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "slug", label: "Slug", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "content", label: "Content", kind: FieldKind::Textarea, hidden: false, readonly: false },
        ]
    }
}
"##;
    manifest.push(("src/models/post.rs", post_model.to_string()));

    let models_mod = r##"pub mod post;
"##;
    manifest.push(("src/models/mod.rs", models_mod.to_string()));

    // 4. Controller
    let repo_import = if is_repo {
        "use crate::repositories::post_repository::PostRepository;"
    } else {
        "use crate::models::post::Post;"
    };
    let all_call = if is_repo {
        "PostRepository::find_all().await.unwrap_or_default()"
    } else {
        "Post::all().await.unwrap_or_default()"
    };

    let blog_controller = format!(
        r##"use rullst::server::{{Path, IntoResponse}};
use rullst::response::Html;
{repo_import}
use crate::pages::blog;

pub async fn index() -> impl IntoResponse {{
    let posts = {all_call};
    Html(blog::index_page(posts))
}}

pub async fn show(Path(slug): Path<String>) -> impl IntoResponse {{
    let posts = {all_call};
    let Some(post) = posts.into_iter().find(|post| post.slug == slug) else {{
        return (
            rullst::http::StatusCode::NOT_FOUND,
            "Blog post not found",
        )
            .into_response();
    }};
    Html(blog::detail_page(post)).into_response()
}}

pub async fn robots_txt() -> impl IntoResponse {{
    (
        rullst::http::StatusCode::OK,
        "User-agent: *\nDisallow: /nexus\nSitemap: /sitemap.xml\n",
    )
}}

pub async fn sitemap_xml() -> impl IntoResponse {{
    (
        rullst::http::StatusCode::OK,
        [(rullst::http::header::CONTENT_TYPE, "application/xml")],
        r#"<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"><url><loc>/</loc></url></urlset>"#,
    )
}}
"##,
        repo_import = repo_import,
        all_call = all_call,
    );
    manifest.push(("src/controllers/blog_controller.rs", blog_controller));

    let controllers_mod = r##"pub mod blog_controller;
"##;
    manifest.push(("src/controllers/mod.rs", controllers_mod.to_string()));

    // Repository layer (if applicable)
    if is_repo {
        manifest.push((
            "src/repositories/post_repository.rs",
            common::generate_repository("Post", "posts"),
        ));
        manifest.push((
            "src/repositories/mod.rs",
            common::generate_repositories_mod(&["Post"]),
        ));
    }

    // 5. Pages
    let page_header = common::frontend_page_imports(frontend_engine);
    let blog_page_body = r##"use crate::models::post::Post;

pub fn index_page(posts: Vec<Post>) -> String {
    html! {
        <html lang="en" class="dark">
            <head>
            <link rel="icon" type="image/png" href="https://raw.githubusercontent.com/venelouis/Rullst/main/Rullst.png" />
                <meta charset="UTF-8" />
                <title>"Rullst Press Feed"</title>
                <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700&display=swap" rel="stylesheet" />
                <style>
                    "
                    * { box-sizing: border-box; margin: 0; padding: 0; font-family: 'Outfit', sans-serif; }
                    body { background: #030712; color: #f3f4f6; min-height: 100vh; padding: 4rem 2rem; display: flex; flex-direction: column; align-items: center; }
                    .container { max-width: 800px; width: 100%; }
                    header { text-align: center; margin-bottom: 5rem; }
                    h1 { font-size: 3.5rem; font-weight: 800; background: linear-gradient(135deg, #059669, #f97316); -webkit-background-clip: text; -webkit-text-fill-color: transparent; }
                    p.sub { color: #9ca3af; font-size: 1.20rem; margin-top: 0.5rem; }
                    .post-list { display: flex; flex-direction: column; gap: 2.5rem; }
                    .card { background: rgba(17, 24, 39, 0.7); backdrop-filter: blur(12px); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 20px; padding: 2.5rem; transition: transform 0.2s, border-color 0.2s; }
                    .card:hover { transform: translateY(-3px); border-color: rgba(5, 150, 105, 0.4); }
                    .card h2 { font-size: 1.75rem; color: #ffffff; margin-bottom: 1rem; }
                    .card p { color: #9ca3af; font-size: 1rem; line-height: 1.7; margin-bottom: 1.5rem; }
                    .read-more { color: #f97316; text-decoration: none; font-weight: 600; font-size: 0.95rem; }
                    .read-more:hover { text-decoration: underline; }
                    "
                </style>
            </head>
            <body>
                <div class="container">
                    <header style="display: flex; justify-content: space-between; align-items: center;">
                        <div style="text-align: left;">
                            <h1>"RullstPress Feed"</h1>
                            <p class="sub">"Insights on hyper-performance fullstack development"</p>
                        </div>
                        <div style="display: flex; gap: 1rem; align-items: flex-start;">
                        <div style="display: flex; flex-direction: column; align-items: center; gap: 0.25rem;">
                            <a href="/nexus" style="background: rgba(5, 150, 105, 0.2); border: 1px solid rgba(5, 150, 105, 0.5); color: #10b981; padding: 0.5rem 1rem; border-radius: 0.5rem; text-decoration: none; font-weight: 600; font-size: 0.9rem;">"⚙️ Nexus CMS"</a>
                            <span style="font-size: 0.7rem; color: #94a3b8;">"(local in debug; credentials in release)"</span>
                        </div>
                            <a href="http://127.0.0.1:5555" target="_blank" style="background: rgba(249, 115, 22, 0.2); border: 1px solid rgba(249, 115, 22, 0.5); color: #f97316; padding: 0.5rem 1rem; border-radius: 0.5rem; text-decoration: none; font-weight: 600; font-size: 0.9rem;">"📊 Rullst Studio (local)"</a>
                        </div>
                    </header>
                    <div class="post-list">
                        { rullst::html::RawHtml::new(posts.into_iter().map(|p| html! {
                            <div class="card">
                                <h2>{&p.title}</h2>
                                <p>{p.content.chars().take(100).collect::<String>()} "..."</p>
                                <a class="read-more" href={format!("/posts/{}", p.slug)}>"Read full post &rarr;"</a>
                            </div>
                        }).collect::<Vec<_>>().join("")) }
                    </div>
                </div>
            </body>
        </html>
    }
}

pub fn detail_page(post: Post) -> String {
    html! {
        <html lang="en" class="dark">
            <head>
            <link rel="icon" type="image/png" href="https://raw.githubusercontent.com/venelouis/Rullst/main/Rullst.png" />
                <meta charset="UTF-8" />
                <title>{&post.title}</title>
                <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700&display=swap" rel="stylesheet" />
                <style>
                    "
                    * { box-sizing: border-box; margin: 0; padding: 0; font-family: 'Outfit', sans-serif; }
                    body { background: #030712; color: #f3f4f6; min-height: 100vh; padding: 4rem 2rem; display: flex; flex-direction: column; align-items: center; }
                    .container { max-width: 700px; width: 100%; }
                    .back-link { color: #f97316; text-decoration: none; font-weight: 600; margin-bottom: 2rem; display: inline-block; }
                    h1 { font-size: 3rem; font-weight: 800; color: #ffffff; margin-bottom: 2rem; line-height: 1.2; }
                    .content { font-size: 1.15rem; color: #d1d5db; line-height: 1.8; }
                    "
                </style>
            </head>
            <body>
                <div class="container">
                    <a class="back-link" href="/">"← Back to Feed"</a>
                    <h1>{&post.title}</h1>
                    <div class="content">
                        {&post.content}
                    </div>
                </div>
            </body>
        </html>
    }
}
"##;
    let blog_page = page_header + blog_page_body;
    manifest.push(("src/pages/blog.rs", blog_page));

    let pages_mod = r##"pub mod blog;
"##;
    manifest.push(("src/pages/mod.rs", pages_mod.to_string()));

    manifest
}
