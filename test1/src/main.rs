pub mod migrations;
pub mod models;
pub mod controllers;
pub mod pages;

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    rullst::artisan!(crate::migrations::get_migrations());

    let is_dev = std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string()) != "production";
    if is_dev {
        rullst::runtime::spawn(async { let _ = rullst::studio::run_studio("").await; });
        println!("📊 Rullst Studio running on port 5555");
    }
    println!("🚀 Blog server starting on port 3000...");
    let is_hot = std::env::var("HOT_RELOAD").is_ok();

    let server = if is_hot {
        let lib_path = if cfg!(target_os = "windows") {
            format!("target/debug/{}", "test1")
        } else {
            format!("target/debug/lib{}", "test1")
        };
        rullst::Server::new_hot(&lib_path)
    } else {
        let router_ptr = test1::rullst_router_init();
        let router = unsafe { *Box::from_raw(router_ptr) };
        rullst::Server::new(router)
    };

    server.run(3000).await?;

    Ok(())
}
