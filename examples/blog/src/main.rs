#![allow(unexpected_cfgs)]
#![cfg_attr(mutants, mutants::skip)]
use rullst::{Server, multitenant};
use rullst_blog_example::app::Post;
use rullst_orm::Orm;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Intercept Artisan and Studio CLI commands
    rullst::artisan!(vec![]);

    // Initialize SQLite database
    Orm::init("sqlite://blog.db").await?;

    // Create table schema
    let pool = Orm::pool();
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

    // Clean old startup/enterprise seeds if migrating
    let _ = rullst::db::sqlx::query(
        "DELETE FROM posts WHERE title LIKE 'Enterprise Architecture%' OR title LIKE 'High-Velocity MVP%'"
    )
    .execute(pool)
    .await;

    // Seed Sovereign SaaS Blog Posts
    let _ = multitenant::TENANT_CONTEXT
        .scope(std::cell::RefCell::new(Some("community".to_string())), async {
            // 1. Unified Welcome & Overview Post
            let welcome_exists = Post::query()
                .where_eq("title", "Welcome to The Sovereign SaaS Blog & Publisher")
                .first()
                .await
                .unwrap_or(None)
                .is_some();

            if !welcome_exists {
                let mut post1 = Post {
                    id: 0,
                    tenant_id: "community".to_string(),
                    title: "Welcome to The Sovereign SaaS Blog & Publisher".to_string(),
                    body: "Welcome to The Sovereign SaaS Blog & Publisher. Explore the top navigation bar to test all front-end paradigms, the Hybrid ORM, Rullst Studio (/studio), Nexus CMS (/nexus), Capital Billing, and Security RASP in action!\n\nUnder Rullst SaaS Multi-tenancy with Task-Local Scopes, database records are strictly isolated with zero cross-tenant leakage. Startups and enterprise teams can build full-featured reactive applications in pure Rust with high velocity without maintaining complex JavaScript npm ecosystems.".to_string(),
                };
                let _ = post1.save().await;
            }

            // 2. Architecture Comparison: The 5 Frontend Paradigms in Rullst
            let comparison_exists = Post::query()
                .where_eq("title", "Architecture Deep Dive: The 5 Frontend Paradigms in Rullst")
                .first()
                .await
                .unwrap_or(None)
                .is_some();

            if !comparison_exists {
                let mut post2 = Post {
                    id: 0,
                    tenant_id: "community".to_string(),
                    title: "Architecture Deep Dive: The 5 Frontend Paradigms in Rullst".to_string(),
                    body: "Here is an in-depth breakdown of how Rullst unifies all 5 web paradigms natively, eliminating framework lock-in:\n\n1. ⚡ Zero-Bundle HTMX SSR (Rullst Native Standard):\n- Paradigm: Declarative HTML5 attributes with compile-time `html!` macro.\n- Footprint: 0 KB JavaScript bundle. Sub-millisecond TTFB.\n- Ideal For: SEO landing pages, SaaS dashboards, and CRUD apps.\n\n2. 🔴 LiveView Server-Driven UI (`rullst::live` — Phoenix & Dioxus pattern):\n- Paradigm: Bidirectional WebSocket state synchronization over Tokio.\n- Footprint: Zero client-side logic. State lives in server RAM; DOM diffs are pushed to the browser in real-time.\n- Ideal For: Live feeds, chats, reactive counters, and notifications.\n\n3. 🏝️ Reactive Wasm Islands (`rullst::island` — Leptos & Yew WASM/Signals pattern):\n- Paradigm: Client-side WebAssembly micro-frontends mounted pontually.\n- Footprint: Isolated WASM bytecode running in the browser VM.\n- Ideal For: Rich Markdown editors, canvas games, offline calculations, and charting.\n\n4. 🎨 Zero-Build Pure CSS (Adobe Topcoat pattern — `/topcoat-demo`):\n- Paradigm: 60 FPS GPU-accelerated CSS components without Node.js or NPM.\n- Footprint: 0 KB JS, instant loading, dark/light themes out of the box.\n- Ideal For: Backend developers wanting clean UIs with zero JS build pipelines.\n\n5. 📄 File-Based Classic Templates (Jinja2 / Tera Engine — Loco & Rails pattern — `/templates-demo`):\n- Paradigm: External HTML files located in `templates/*.html` with layout inheritance.\n- Footprint: Decoupled presentation layer for frontend designers.\n- Ideal For: Teams migrating from Django, Laravel, Rails, or Loco.rs.".to_string(),
                };
                let _ = post2.save().await;
            }
        })
        .await;

    let is_hot = std::env::var("HOT_RELOAD").is_ok();

    let server = if is_hot {
        let lib_path = if cfg!(target_os = "windows") {
            if std::path::Path::new("target/debug/rullst_blog_example.dll").exists() {
                "target/debug/rullst_blog_example"
            } else {
                "../../target/debug/rullst_blog_example"
            }
        } else {
            if std::path::Path::new("target/debug/librullst_blog_example.so").exists()
                || std::path::Path::new("target/debug/librullst_blog_example.dylib").exists()
            {
                "target/debug/librullst_blog_example"
            } else {
                "../../target/debug/librullst_blog_example"
            }
        };
        Server::new_hot(lib_path)
    } else {
        let router_ptr = rullst_blog_example::rullst_router_init();
        let router = unsafe { *Box::from_raw(router_ptr) };
        Server::new(router)
    };

    println!("🚀 Rullst Sovereign SaaS Showcase running at http://127.0.0.1:3000");
    println!("   - Studio Developer Control Room: http://127.0.0.1:3000/studio");
    println!("   - Nexus Admin CMS: http://127.0.0.1:3000/nexus");

    server.run(3000).await?;

    Ok(())
}
