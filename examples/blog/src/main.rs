#![allow(unexpected_cfgs)]
#![cfg_attr(mutants, mutants::skip)]
use rullst::{Server, multitenant};
use rullst_blog_example::app::Post;
use rullst_orm::Orm;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Intercept Artisan and Studio CLI commands
    rullst::artisan!(vec![]);

    #[cfg(debug_assertions)]
    tokio::spawn(async {
        if let Err(error) = rullst_studio::run_studio(5555).await {
            eprintln!("Rullst Studio could not start: {error}");
        }
    });

    // Initialize SQLite database
    Orm::init("sqlite://blog.db").await?;

    // Create table schema
    let pool = Orm::pool()?;
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
        "DELETE FROM posts WHERE title LIKE 'Enterprise Architecture%' OR title LIKE 'High-Velocity MVP%' OR title LIKE 'Architecture Deep Dive%' OR body LIKE '%Topcoat%'"
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
                    body: "Welcome to The Sovereign SaaS Blog & Publisher. Explore the top navigation bar to test the available front-end foundations, the Hybrid ORM, local Rullst Studio (http://127.0.0.1:5555), Nexus CMS (/nexus), Capital Billing, and Security RASP demonstrations.\n\nThe example uses task-local tenant scoping as a development fixture. Production applications must derive membership from authenticated identity and keep cross-tenant negative tests in their own authorization model.".to_string(),
                };
                let _ = post1.save().await;
            }

            // 2. UI integration patterns represented by this bounded showcase.
            let mut post2 = Post {
                id: 0,
                tenant_id: "community".to_string(),
                title: "Architecture Deep Dive: Five UI Integration Patterns".to_string(),
                body: "This bounded showcase places five UI integration patterns beside the same Rust backend. They are options, not a claim that one abstraction removes every kind of lock-in.\n\n1. ⚡ Server-rendered HTML with optional HTMX:\n- `html!` produces typed server-side markup. This demo loads HTMX from a CDN where partial navigation needs a small browser runtime.\n\n2. 🔴 Server-driven WebSocket UI (`rullst::live`):\n- State mutations execute on the Tokio server while HTMX and its WebSocket extension carry events and apply returned markup in the browser.\n\n3. 🏝️ WebAssembly islands (`rullst::island`):\n- Selected client behavior can compile to WebAssembly; payload size and responsiveness remain application measurements.\n\n4. 🎨 Semantic CSS (`/pico-demo`):\n- Pico.css styles ordinary HTML without a Node.js build pipeline. This showcase still uses small inline browser handlers for its interactive controls.\n\n5. 📄 Embedded file templates (`/templates-demo`):\n- An external HTML file is embedded and populated by a deliberately small fixture. It demonstrates file separation, not the full Tera language or runtime.".to_string(),
            };
            let _ = post2.save().await;
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
        let router = rullst_blog_example::router()?;
        Server::new(router)
    };

    println!("🚀 Rullst Sovereign SaaS Showcase running at http://127.0.0.1:3000");
    #[cfg(debug_assertions)]
    println!("   - Studio Developer Control Room: http://127.0.0.1:5555");
    println!("   - Nexus Admin CMS: http://127.0.0.1:3000/nexus");

    server.run(3000).await?;

    Ok(())
}
