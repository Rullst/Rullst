use sqlx::any::AnyPoolOptions;
use sqlx::any::install_default_drivers;

#[tokio::main]
async fn main() {
    install_default_drivers();
    match AnyPoolOptions::new().connect("sqlite::memory:").await {
        Ok(_) => println!("Success sqlite::memory:"),
        Err(e) => println!("Error: {:?}", e),
    }
}
