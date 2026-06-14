pub mod controllers;
pub mod pages;

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {


    println!("🚀 AI Portfolio server starting on port 3000...");
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
