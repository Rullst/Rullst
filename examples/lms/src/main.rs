use rullst::Server;

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    rullst::artisan!(rullst_lms_example::migrations::get_migrations());
    #[cfg(debug_assertions)]
    {
        rullst::runtime::spawn(async {
            if let Err(error) = rullst::studio::run_studio(5555).await {
                eprintln!("Rullst Studio could not start: {error}");
            }
        });
    }
    Server::new(rullst_lms_example::router()?)
        .with_migrations(rullst_lms_example::migrations::get_migrations())
        // Production monitoring is off. To serve the read-only
        // `GET /_rullst/monitor/v1` document, uncomment the next lines and set
        // RULLST_MONITOR_TOKENS (see .env.example); without a token nothing is
        // mounted. Guide: https://rullst.github.io/Rullst/book/production-monitoring.html
        // .with_monitor(
        //     rullst::monitor::Monitor::new()
        //         .with_application(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
        // )
        .run(3000)
        .await?;
    Ok(())
}
