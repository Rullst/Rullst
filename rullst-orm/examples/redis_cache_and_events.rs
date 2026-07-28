use rullst_orm::{FromRow, Orm};
#[cfg(feature = "redis")]
use std::time::Duration;
#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "products")]
pub struct Product {
    pub id: i32,
    pub name: String,
    pub price: f64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Setup local sqlite database
    let _ = std::fs::remove_file("products_demo.db");
    std::fs::File::create("products_demo.db")?;

    Orm::init("sqlite://products_demo.db").await?;
    let pool = Orm::pool();

    rullst_orm::_sqlx::query(
        "CREATE TABLE products (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, price REAL NOT NULL)"
    )
    .execute(pool)
    .await?;

    // 2. Initialize Redis Connection Manager
    // (Assuming a local Redis server is running on 127.0.0.1:6379)
    let redis_url = "redis://127.0.0.1:6379";
    println!("Connecting to Redis at {}...", redis_url);

    #[cfg(feature = "redis")]
    {
        if let Err(e) = Orm::init_redis(redis_url).await {
            println!(
                "⚠️ Could not connect to Redis: {}. Skipping Redis caching and Pub/Sub event demo.",
                e
            );
            let _ = std::fs::remove_file("products_demo.db");
            return Ok(());
        }
        println!("✅ Connected to Redis successfully!");

        // 3. Spawn a background thread to subscribe to all product events and print them
        tokio::spawn(async move {
            if let Ok(client) = Orm::redis_client()
                && let Ok(mut conn) = client.get_connection()
            {
                let mut pubsub = conn.as_pubsub();
                let _ = pubsub.psubscribe("orm:events:products:*");
                println!(
                    "📡 Background Subscriber: Listening for products Pub/Sub events on Redis..."
                );
                loop {
                    if let Ok(msg) = pubsub.get_message() {
                        let channel: String = msg.get_channel_name().to_string();
                        let payload: String = msg.get_payload().unwrap_or_default();
                        println!(
                            "🔔 [Pub/Sub Event] Received event on channel '{}': {}",
                            channel, payload
                        );
                    }
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            }
        });

        tokio::time::sleep(Duration::from_millis(200)).await;

        // 4. Save a new product (Routes to DB + Publishes created and saved events to Redis!)
        println!("\n📥 Saving a new product to database...");
        let mut p = Product {
            id: 0,
            name: "Super Quantum Laptop".to_string(),
            price: 2499.99,
        };
        p.save().await?;
        println!("✅ Product saved with ID: {}", p.id);

        tokio::time::sleep(Duration::from_millis(200)).await;

        // Enable query logging to verify cache hits
        Orm::enable_query_log();

        // 5. Caching: fetch with .remember(10) (10 seconds cache TTL)
        println!(
            "\n🔍 Fetching product for the FIRST time (should hit SQL database and cache in Redis):"
        );
        let p_db = Product::query().where_id(p.id).remember(10).first().await?;
        println!("Fetched product: {:?}", p_db);

        println!(
            "\n⚡ Fetching product for the SECOND time (should hit cache instantly, no SQL log!):"
        );
        let p_cached = Product::query().where_id(p.id).remember(10).first().await?;
        println!("Fetched from cache: {:?}", p_cached);

        // 6. Delete product (Routes to DB + Publishes deleted event!)
        println!("\n🗑️ Deleting product...");
        p.delete().await?;
        println!("✅ Product deleted successfully!");

        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    #[cfg(not(feature = "redis"))]
    {
        println!("⚠️ Caching & Event features require '--features redis' flag to run!");
    }

    // Clean up
    let _ = std::fs::remove_file("products_demo.db");
    println!("\n🎉 Redis Caching and Pub/Sub event demo completed successfully!");
    Ok(())
}
