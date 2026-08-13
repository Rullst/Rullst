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

            // 2. Architecture Comparison: Rullst vs Leptos vs Dioxus Post
            let comparison_exists = Post::query()
                .where_eq("title", "Architecture Deep Dive: Rullst vs Leptos vs Dioxus")
                .first()
                .await
                .unwrap_or(None)
                .is_some();

            if !comparison_exists {
                let mut post2 = Post {
                    id: 0,
                    tenant_id: "community".to_string(),
                    title: "Architecture Deep Dive: Rullst vs Leptos vs Dioxus".to_string(),
                    body: "Here is an in-depth breakdown of the 3 front-end paradigms supported and demonstrated in Rullst:\n\n1. ⚡ HTMX SSR (Rullst Native Standard):\n- Core Philosophy: Pure Server-Side Rendering with compile-time `html!` macro.\n- Client Footprint: Zero JavaScript bundle overhead. Microsecond Time-to-First-Byte (TTFB).\n- Best For: Ultra-fast SEO-rich dashboards, SaaS backends, minimal battery and RAM usage on user devices.\n\n2. 🏝️ Wasm Island (Leptos Pattern):\n- Core Philosophy: High-performance WebAssembly (.wasm) binary compiled via `wasm-bindgen`.\n- Client Footprint: Client-side execution inside the browser's WebAssembly VM with fine-grained reactive Signals.\n- Best For: Complex in-browser tools, rich graphics, video processors, and zero-latency client computations.\n\n3. 🔴 LiveView WS (Dioxus Pattern):\n- Core Philosophy: Server-Driven UI synchronized in real-time over persistent WebSockets.\n- Client Footprint: Zero client state logic. State resides entirely in Tokio server memory; state mutations trigger automatic DOM diff patches pushed to the browser.\n- Best For: Real-time feeds, live chat, interactive collaboration, and event-driven admin dashboards.".to_string(),
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
