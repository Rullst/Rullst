use rullst::Server;

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    rullst::artisan!(rullst_portfolio_example::migrations::get_migrations());

    #[cfg(debug_assertions)]
    {
        rullst::runtime::spawn(async {
            if let Err(error) = rullst::studio::run_studio(5555).await {
                eprintln!("Rullst Studio could not start: {error}");
            }
        });
        println!("📊 Rullst Studio running on http://127.0.0.1:5555");
    }

    let router = rullst_portfolio_example::router()?;

    println!(
        "🚀 AI Portfolio server starting on port 3000 (Engine: Zero-Bundle HTMX, ORM: Active Record)..."
    );
    println!(
        "⚙️  Nexus CMS: http://127.0.0.1:3000/nexus (local loopback access in debug; environment credentials in release)"
    );
    Server::new(router)
        .with_migrations(rullst_portfolio_example::migrations::get_migrations())
        .run(3000)
        .await?;

    Ok(())
}
