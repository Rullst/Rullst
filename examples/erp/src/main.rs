use rullst::Server;

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Runs an Artisan command such as `db:migrate` and exits; a plain start applies no migrations.
    rullst::artisan!(rullst_erp_example::migrations::get_migrations());

    let router = rullst_erp_example::router()?;

    #[cfg(debug_assertions)]
    {
        rullst::runtime::spawn(async {
            if let Err(error) = rullst::studio::run_studio(5555).await {
                eprintln!("Rullst Studio could not start: {error}");
            }
        });
        println!("📊 Rullst Studio running on http://127.0.0.1:5555");
    }
    println!("🚀 ERP Pocket server starting on port 3000...");
    Server::new(router)
        .with_migrations(rullst_erp_example::migrations::get_migrations())
        .run(3000)
        .await?;

    Ok(())
}
