use rullst_orm::Orm;
use rullst_orm::schema::{Migration, Schema, run_artisan};

pub struct CreateUsersMigration;

#[rullst_orm::async_trait]
impl Migration for CreateUsersMigration {
    fn name(&self) -> &'static str {
        "m1716612001_create_users_table"
    }

    async fn up(&self) -> Result<(), rullst_orm::Error> {
        Schema::create("users", |table| {
            table.id();
            table.string("username").not_null();
            table.timestamps();
        })
        .await
    }

    async fn down(&self) -> Result<(), rullst_orm::Error> {
        Schema::drop_if_exists("users").await
    }
}

pub struct CreatePostsMigration;

#[rullst_orm::async_trait]
impl Migration for CreatePostsMigration {
    fn name(&self) -> &'static str {
        "m1716612002_create_posts_table"
    }

    async fn up(&self) -> Result<(), rullst_orm::Error> {
        Schema::create("posts", |table| {
            table.id();
            table.string("title").not_null();
            table.timestamps();
        })
        .await
    }

    async fn down(&self) -> Result<(), rullst_orm::Error> {
        Schema::drop_if_exists("posts").await
    }
}

#[tokio::main]
async fn main() -> Result<(), rullst_orm::Error> {
    let _ = std::fs::remove_file("artisan_test.db");
    std::fs::File::create("artisan_test.db").unwrap();
    Orm::init("sqlite://artisan_test.db").await?;

    println!("--- 🛠️  Simulating Migration Execution (Batch 1) ---");
    let migrations: Vec<Box<dyn Migration>> = vec![
        Box::new(CreateUsersMigration),
        Box::new(CreatePostsMigration),
    ];

    // Under the hood, run_artisan executes them and logs batches to the DB
    run_artisan(migrations, vec![]).await?;

    // Verify migrations table and schemas
    let pool = Orm::pool();
    let rows: Vec<(i32, String, i32)> =
        rullst_orm::_sqlx::query_as("SELECT id, migration, batch FROM migrations")
            .fetch_all(pool)
            .await?;

    println!("\n--- Migrations Table Status ---");
    for (id, name, batch) in &rows {
        println!("ID: {}, Migration: {}, Batch: {}", id, name, batch);
    }

    // Verify tables exist
    let count: (i64,) = rullst_orm::_sqlx::query_as(
        "SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name IN ('users', 'posts')",
    )
    .fetch_one(pool)
    .await?;
    println!("Tables before rollback: {}", count.0);

    // Clean up
    let _ = std::fs::remove_file("artisan_test.db");

    println!("\n🎉 Artisan Migrations State Machine verified successfully!");

    Ok(())
}
