pub mod migrations;

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Intercept Artisan commands (e.g. cargo rullst db:migrate) before starting server
    rullst::artisan!(crate::migrations::get_migrations());

    let is_hot = std::env::var("HOT_RELOAD").is_ok();

    let server = if is_hot {
        let lib_path = if cfg!(target_os = "windows") {
            "target/debug/test1"
        } else {
            "target/debug/libtest1"
        };
        rullst::Server::new_hot(lib_path)
    } else {
        let router_ptr = test1::rullst_router_init();
        let router = unsafe { *Box::from_raw(router_ptr) };
        rullst::Server::new(router)
    };

    server.run(3000).await?;

    Ok(())
}
