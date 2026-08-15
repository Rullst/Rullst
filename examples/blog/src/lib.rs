#![allow(clippy::needless_update)]
#![allow(unexpected_cfgs)]
#![cfg_attr(mutants, mutants::skip)]

pub mod ai_demo;
pub mod billing_demo;
pub mod interactive_counter;
pub mod omni_demo;
pub mod repository_demo;
pub mod security_demo;
pub mod showcase_nav;

#[cfg(not(target_arch = "wasm32"))]
pub mod live_counter;

#[cfg(not(target_arch = "wasm32"))]
pub mod app {
    use crate::live_counter::CounterComponent;
    use crate::showcase_nav::{render_shared_styles, render_showcase_nav};
    use axum::Form;
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

    /// Zero-Bundle HTMX SSR Landing Page (`/`)
    pub async fn index() -> impl IntoResponse {
        let posts = Post::all().await.unwrap_or_default();
        let nav = render_showcase_nav("/");
        let styles = render_shared_styles();
        let post_list_html = render_post_list(&posts);

        Html(html! {
            <html lang="en">
                <head>
                    <meta charset="utf-8" />
                    <title>"Rullst Sovereign SaaS Blog & Publisher"</title>
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
                                        "⚡ Zero-Bundle HTMX Server-Side Rendering"
                                        <span class="feature-tag tag-orm">"rullst-core"</span>
                                    </h1>
                                    <p style="color: var(--text-muted); margin-bottom: 1.5rem;">
                                        "Ultra-fast declarative UI generated with zero client-side bundle overhead. Powered by Rullst's compile-time `html!` macro and Axum static dispatch."
                                    </p>
                                </div>
                            </div>

                            <form method="post" action="/posts" style="background: #05070c; border: 1px solid #1e293b; border-radius: 0.5rem; padding: 1.5rem;">
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

    /// LiveView WebSocket Feed Page (`/live-feed` and `/live-counter`)
    pub async fn live_demo() -> impl IntoResponse {
        let nav = render_showcase_nav("/live-feed");
        let styles = render_shared_styles();
        let component_mount = rullst::live::Live::mount::<CounterComponent>("/_live").await;

        Html(html! {
            <html lang="en">
            <head>
                <meta charset="utf-8" />
                <title>"Rullst LiveView - Real-time WebSockets Feed"</title>
                <link rel="icon" type="image/png" href="https://raw.githubusercontent.com/venelouis/Rullst/main/Rullst.png" />
                <style>{ rullst::html::RawHtml(styles) }</style>
                <script src="https://unpkg.com/htmx.org@1.9.12"></script>
                <script src="https://unpkg.com/htmx.org@1.9.12/dist/ext/ws.js"></script>
            </head>
            <body>
                { rullst::html::RawHtml(nav) }
                <div class="container">
                    <div class="card">
                        <h1 class="card-title">
                            "🔴 LiveView Server-Driven UI"
                            <span class="feature-tag tag-ai">"rullst::live"</span>
                        </h1>
                        <p style="color: var(--text-muted); margin-bottom: 1.5rem;">
                            "Zero client-side state JavaScript. All state mutations and event handlers execute on Tokio threads in pure Rust, synchronizing DOM patches over WebSockets."
                        </p>

                        <div style="background: #05070c; border: 1px solid #1e293b; border-radius: 0.5rem; padding: 2rem; text-align: center;">
                            { rullst::html::RawHtml(component_mount) }
                        </div>
                    </div>
                </div>
            </body>
            </html>
        })
    }

    /// Wasm Island Reactive Editor Page (`/editor`)
    pub async fn wasm_demo() -> impl IntoResponse {
        let nav = render_showcase_nav("/editor");
        let styles = render_shared_styles();
        let component_mount = crate::interactive_counter::InteractiveCounter(42);

        Html(html! {
            <html lang="en">
            <head>
                <meta charset="utf-8" />
                <title>"Rullst Wasm Island - Client-side Reactive WebAssembly"</title>
                <link rel="icon" type="image/png" href="https://raw.githubusercontent.com/venelouis/Rullst/main/Rullst.png" />
                <style>{ rullst::html::RawHtml(styles) }</style>
            </head>
            <body>
                { rullst::html::RawHtml(nav) }
                <div class="container">
                    <div class="card">
                        <h1 class="card-title">
                            "🏝️ Wasm Island Architecture"
                            <span class="feature-tag tag-orm">"wasm-bindgen"</span>
                        </h1>
                        <p style="color: var(--text-muted); margin-bottom: 1.5rem;">
                            "Islands of interactivity compiled directly from Rust to WebAssembly with zero VDOM overhead."
                        </p>

                        <div style="background: #05070c; border: 1px solid #1e293b; border-radius: 0.5rem; padding: 2rem; text-align: center;">
                            { rullst::html::RawHtml(component_mount) }
                        </div>

                        <script type="module">
                            "import init from '/static/rullst_blog_example.js'; init();"
                        </script>
                    </div>
                </div>
            </body>
            </html>
        })
    }

    /// WebSocket handler for LiveView
    pub async fn live_ws(ws: axum::extract::ws::WebSocketUpgrade) -> impl IntoResponse {
        rullst::live::live_ws_handler::<CounterComponent>(ws).await
    }

    /// Honeypot sensor endpoint (`/wp-admin`)
    pub async fn honeypot_trap() -> impl IntoResponse {
        tracing::warn!("🚨 Honeypot trap triggered on /wp-admin! IP logged to threat radar.");
        (
            axum::http::StatusCode::FORBIDDEN,
            [(axum::http::header::CONTENT_TYPE, "text/plain")],
            "Access Denied: Incident logged in Rullst SOC Threat Radar.",
        )
    }

    pub async fn favicon_handler() -> impl IntoResponse {
        Redirect::temporary("https://raw.githubusercontent.com/venelouis/Rullst/main/Rullst.png")
    }

    pub async fn robots_txt() -> impl IntoResponse {
        (
            axum::http::StatusCode::OK,
            "User-agent: *\nDisallow: /studio\nDisallow: /nexus\n",
        )
    }

    pub async fn sitemap_xml() -> impl IntoResponse {
        (
            axum::http::StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "application/xml")],
            r#"<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"><url><loc>http://localhost:3000/</loc></url></urlset>"#,
        )
    }

    pub async fn set_security_headers(
        mut response: axum::response::Response,
    ) -> axum::response::Response {
        let headers = response.headers_mut();
        headers.insert(
            "Content-Security-Policy",
            "default-src 'self' https://unpkg.com https://cdn.tailwindcss.com https://fonts.googleapis.com https://fonts.gstatic.com https://raw.githubusercontent.com data:; script-src 'self' 'unsafe-inline' 'unsafe-eval' https://unpkg.com https://cdn.tailwindcss.com; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com https://cdn.tailwindcss.com; font-src 'self' https://fonts.gstatic.com; img-src 'self' data: https://raw.githubusercontent.com https://*.githubusercontent.com; connect-src 'self' ws: wss:;".parse().unwrap(),
        );
        headers.insert(
            "Cross-Origin-Resource-Policy",
            "cross-origin".parse().unwrap(),
        );
        headers.insert("X-Content-Type-Options", "nosniff".parse().unwrap());
        headers.insert("X-Frame-Options", "SAMEORIGIN".parse().unwrap());
        headers.insert("X-XSS-Protection", "1; mode=block".parse().unwrap());
        response
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[unsafe(no_mangle)]
pub extern "C" fn rullst_router_init() -> *mut rullst::Router {
    use app::*;
    use rullst::routes;

    let config =
        rullst::TenantConfig::new(rullst::TenantStrategy::Header).with_header_name("X-Tenant-ID");

    let studio_router = rullst_studio::Studio::new().into_router();
    let nexus_router = rullst_nexus::Nexus::new()
        .with_brand("Rullst Sovereign Publisher")
        .register::<Post>()
        .build();

    let router = routes![
        get("/" => index),
        post("/posts" => store),
        get("/posts/repository" => crate::repository_demo::repository_page),
        get("/editor" => wasm_demo),
        get("/live-feed" => live_demo),
        get("/live-counter" => live_demo),
        get("/_live" => live_ws),
        get("/wasm-counter" => wasm_demo),
        get("/pricing" => crate::billing_demo::pricing_page),
        get("/billing" => crate::billing_demo::pricing_page),
        get("/checkout" => crate::billing_demo::checkout_handler),
        get("/security-demo" => crate::security_demo::security_page),
        get("/ai-assistant" => crate::ai_demo::ai_page),
        get("/omni" => crate::omni_demo::omni_page),
        get("/wp-admin" => honeypot_trap),
        get("/favicon.ico" => favicon_handler),
        get("/robots.txt" => robots_txt),
        get("/sitemap.xml" => sitemap_xml),
    ]
    .nest_axum("/studio", studio_router)
    .nest_axum("/nexus", nexus_router)
    .layer(axum::middleware::map_response(set_security_headers))
    .layer(rullst::tenant_layer(config));

    Box::into_raw(Box::new(router))
}
