#![allow(unexpected_cfgs)]
#![cfg_attr(mutants, mutants::skip)]
use rullst::Server;
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

    // Select the database exactly as `Server` and the Artisan commands do
    // (DATABASE_URL, then `.env`, then `Rullst.toml`), so all three share it.
    let database_url = rullst_blog_example::database::database_url().await?;
    Orm::init(&database_url).await?;

    rullst_blog_example::app::create_schema().await?;
    rullst_blog_example::app::seed_showcase_posts().await?;

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

    let server = server.with_db(database_url);

    println!("🚀 Rullst Sovereign SaaS Showcase running at http://127.0.0.1:3000");
    #[cfg(debug_assertions)]
    println!("   - Studio Developer Control Room: http://127.0.0.1:5555");
    println!("   - Nexus Admin CMS: http://127.0.0.1:3000/nexus");

    server.run(3000).await?;

    Ok(())
}
