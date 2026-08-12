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

    // Seed post for tenant-enterprise
    let _ = multitenant::TENANT_CONTEXT
        .scope(std::cell::RefCell::new(Some("tenant-enterprise".to_string())), async {
            if Post::all().await.unwrap_or_default().is_empty() {
                let mut post = Post {
                    id: 0,
                    tenant_id: "tenant-enterprise".to_string(),
                    title: "Enterprise Architecture & Rust Scalability".to_string(),
                    body: "Welcome to the Enterprise tenant. Under Rullst SaaS Multi-tenancy with Task-Local Scopes, this record is isolated with zero database leakage across tenants.".to_string(),
                };
                let _ = post.save().await;
            }
        })
        .await;

    // Seed post for tenant-startup
    let _ = multitenant::TENANT_CONTEXT
        .scope(std::cell::RefCell::new(Some("tenant-startup".to_string())), async {
            if Post::all().await.unwrap_or_default().is_empty() {
                let mut post = Post {
                    id: 0,
                    tenant_id: "tenant-startup".to_string(),
                    title: "High-Velocity MVP Delivery with Zero Bundle HTMX".to_string(),
                    body: "Startups can build full-featured reactive applications with Rullst without maintaining complex JavaScript npm ecosystems.".to_string(),
                };
                let _ = post.save().await;
            }
        })
        .await;

    // Seed post for community
    let _ = multitenant::TENANT_CONTEXT
        .scope(std::cell::RefCell::new(Some("community".to_string())), async {
            if Post::all().await.unwrap_or_default().is_empty() {
                let mut post = Post {
                    id: 0,
                    tenant_id: "community".to_string(),
                    title: "Welcome to The Sovereign SaaS Blog & Publisher".to_string(),
                    body: "Explore the top navigation bar to test all 3 Front-End paradigms, the Hybrid ORM, Rullst Studio (/studio), Nexus CMS (/nexus), Capital Billing, and Security RASP in action!".to_string(),
                };
                let _ = post.save().await;
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
