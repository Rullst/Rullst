use rullst_orm::Orm;
use rullst_orm::schema::{Blueprint, ColumnDefault, Schema};

#[tokio::main]
async fn main() -> Result<(), rullst_orm::Error> {
    // 1. Initialize Database
    let _ = std::fs::remove_file("migrations_test.db");
    std::fs::File::create("migrations_test.db").unwrap();
    Orm::init("sqlite://migrations_test.db").await?;

    println!("Creating tables using Fluent Schema Builder...\n");

    // 2. Run Migrations (Create users table)
    Schema::create("users", |table: &mut Blueprint| {
        table.id(); // INTEGER PRIMARY KEY AUTOINCREMENT
        table.string("name").not_null();
        table.string("email").nullable();
        table.integer("age").default(ColumnDefault::Integer(18));
        table
            .boolean("is_active")
            .default(ColumnDefault::Integer(1));
        table.timestamps(); // created_at, updated_at
        table.soft_deletes(); // deleted_at
    })
    .await?;

    println!("Users table created successfully!");

    // 3. Run Migrations (Create posts table)
    Schema::create("posts", |table: &mut Blueprint| {
        table.id();
        table.integer("user_id").not_null();
        table.string("title").not_null();
        table.string("body").nullable();
        table.timestamps();
    })
    .await?;

    println!("Posts table created successfully!");

    // 4. Verification
    let pool = Orm::pool();

    // Let's manually inspect the SQLite sqlite_schema table
    let tables: Vec<(String, String)> = rullst_orm::_sqlx::query_as(
        "SELECT name, sql FROM sqlite_schema WHERE type='table' AND name IN ('users', 'posts')",
    )
    .fetch_all(pool)
    .await?;

    println!("\n--- Generated SQL Schemas ---");
    for (name, sql) in tables {
        println!("Table: {}\n{}\n", name, sql);
    }

    Ok(())
}
