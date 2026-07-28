use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{FromRow, Orm, RullstCollection};

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "products")]
pub struct Product {
    pub id: i32,
    pub name: String,
    pub price: f64,
    pub category: String,
}

#[tokio::main]
async fn main() -> Result<(), rullst_orm::Error> {
    // 1. Initialize DB & Schema
    let _ = std::fs::remove_file("collections_test.db");
    std::fs::File::create("collections_test.db").unwrap();
    Orm::init("sqlite://collections_test.db").await?;

    Schema::create("products", |table: &mut Blueprint| {
        table.id();
        table.string("name").not_null();
        table.float("price").not_null();
        table.string("category").not_null();
    })
    .await?;

    // 2. Insert Test Data
    let products = vec![
        Product {
            id: 0,
            name: "Laptop".to_string(),
            price: 1200.50,
            category: "Tech".to_string(),
        },
        Product {
            id: 0,
            name: "Mouse".to_string(),
            price: 45.00,
            category: "Tech".to_string(),
        },
        Product {
            id: 0,
            name: "Desk".to_string(),
            price: 250.00,
            category: "Furniture".to_string(),
        },
        Product {
            id: 0,
            name: "Chair".to_string(),
            price: 150.00,
            category: "Furniture".to_string(),
        },
    ];

    let pool = Orm::pool();
    let mut query_builder =
        rullst_orm::_sqlx::QueryBuilder::new("INSERT INTO products (name, price, category) ");
    query_builder.push_values(products.into_iter(), |mut b, p| {
        b.push_bind(p.name).push_bind(p.price).push_bind(p.category);
    });
    query_builder.build().execute(pool).await?;

    // 3. Fetch all records using Orm (Returns a standard Vec<Product>)
    let collection = Product::all().await?;

    println!("--- Testing Orm Collections ---\n");

    // 1. implode()
    let names = collection.implode(", ", |p| p.name.clone());
    println!("1. Implode Names: {}", names);

    // 2. sum_by()
    let total_price: f64 = collection.sum_by(|p| p.price);
    println!("2. Total Price of all items: ${:.2}", total_price);

    // 3. max_by_key()
    let most_expensive = collection.max_by_key(|p| (p.price * 100.0) as i64).unwrap(); // Cast float to sortable integer for Ord
    println!(
        "3. Most Expensive Item: {} (${:.2})",
        most_expensive.name, most_expensive.price
    );

    // 4. chunk()
    let chunks = collection.clone().chunk(2);
    println!("4. Chunks of 2:");
    for (i, chunk) in chunks.iter().enumerate() {
        println!(
            "   Chunk {}: {:?}",
            i + 1,
            chunk.iter().map(|p| p.name.clone()).collect::<Vec<_>>()
        );
    }

    // 5. key_by()
    let keyed_by_id = collection.key_by(|p| p.id);
    println!("5. Key By ID:");
    println!("   Product ID 3 is: {}", keyed_by_id.get(&3).unwrap().name);

    Ok(())
}
