#![allow(unexpected_cfgs)]
use rullst::{Server, multitenant};
use rullst_blog_example::app::Post;
use rullst_orm::Orm;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Intercept Artisan and Studio commands
    rullst::artisan!(vec![]);

    // Initialize SQLite file database
    Orm::init("sqlite://blog.db").await?;

    // Create table schema
    let pool = Orm::pool();
    rullst_orm::sqlx::query(
        "CREATE TABLE IF NOT EXISTS posts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tenant_id TEXT NOT NULL,
            title TEXT NOT NULL,
            body TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    // Seed post for tenant1
    let _ = multitenant::TENANT_CONTEXT.scope(std::cell::RefCell::new(Some("tenant1".to_string())), async {
        if Post::all().await.unwrap_or_default().is_empty() {
            let mut post = Post {
                id: 0,
                tenant_id: "tenant1".to_string(),
                title: "Story of Tenant 1".to_string(),
                body: "This is exclusive content for tenant 1. Under Rullst SaaS Multi-tenancy, other tenants cannot view this record!".to_string(),
            };
            let _ = post.save().await;
        }
    }).await;

    // Seed post for tenant2
    let _ = multitenant::TENANT_CONTEXT.scope(std::cell::RefCell::new(Some("tenant2".to_string())), async {
        if Post::all().await.unwrap_or_default().is_empty() {
            let mut post = Post {
                id: 0,
                tenant_id: "tenant2".to_string(),
                title: "Exclusive for Tenant 2".to_string(),
                body: "Only developers authenticated or scoped under tenant 2 will ever load this record.".to_string(),
            };
            let _ = post.save().await;
        }
    }).await;

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

    server.run(3000).await?;

    Ok(())
}
