#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 3007)).await?;
    println!("Wave interference: http://127.0.0.1:3007/webgpu/");
    rullst_core::web::axum::serve(listener, rullst_webgpu_example::router()?)
        .with_graceful_shutdown(async {
            if let Err(error) = tokio::signal::ctrl_c().await {
                eprintln!("Shutdown signal unavailable: {error}");
            }
        })
        .await?;
    Ok(())
}
