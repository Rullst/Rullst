use rullst_orm::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "products")]
pub struct Product {
    pub id: i32,
    pub name: String,
    pub price: f64,
}

#[tokio::main]
async fn main() -> Result<(), rullst_orm::Error> {
    let _ = std::fs::remove_file("query_log_test.db");
    std::fs::File::create("query_log_test.db").unwrap();
    Orm::init("sqlite://query_log_test.db").await?;
    let pool = Orm::pool();

    rullst_orm::_sqlx::query(
        "
        CREATE TABLE products (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            price REAL NOT NULL
        );
    ",
    )
    .execute(pool)
    .await?;

    println!("--- 1. Query Logging is DISABLED by default ---");
    let mut p1 = Product {
        id: 0,
        name: "Premium Mechanical Keyboard".to_string(),
        price: 149.99,
    };
    p1.save().await?;

    let products_count = Product::query().count().await?;
    println!("Products count in DB: {}\n", products_count);

    println!("--- 2. Enabling Query Logging dynamically ---");
    Orm::enable_query_log();

    println!("Saving second product (insert):");
    let mut p2 = Product {
        id: 0,
        name: "Ergonomic Office Chair".to_string(),
        price: 349.50,
    };
    p2.save().await?; // This will print debug info

    println!("\nFetching product with price > 200:");
    let high_value_product = Product::query().where_gt("price", 200.0).first().await?;
    if let Some(ref p) = high_value_product {
        println!("Found product: {} (${})", p.name, p.price);
    }

    println!("\nUpdating price of first product:");
    let mut db_p1 = Product::query().where_eq("id", 1).first().await?.unwrap();
    db_p1.price = 129.99;
    db_p1.save().await?; // This will trigger an update SQL

    println!("\nCounting all products:");
    let count_log = Product::query().count().await?;
    println!("Total products: {}", count_log);

    println!("\nPaginating products (1 per page):");
    let paginated = Product::query().paginate(1, 1).await?;
    println!(
        "Page count: {}, Data len: {}",
        paginated.last_page,
        paginated.data.len()
    );

    println!("\nDeleting all products with price > 300:");
    Product::query()
        .where_gt("price", 300.0)
        .delete_all()
        .await?;

    println!("\n--- 3. Disabling Query Logging dynamically ---");
    Orm::disable_query_log();

    println!("Executing query with logging disabled:");
    let final_count = Product::query().count().await?;
    println!("Final products count: {}", final_count);

    // Clean up
    let _ = std::fs::remove_file("query_log_test.db");
    Ok(())
}
