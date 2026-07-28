use rand::RngExt;
use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{FromRow, Orm, Seeder, async_trait};

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "users")]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
}

// -----------------------------------------------------------------------------
// SEEDER IMPLEMENTATION
// -----------------------------------------------------------------------------
pub struct DatabaseSeeder;

#[async_trait]
impl Seeder for DatabaseSeeder {
    async fn run(&self) -> Result<(), rullst_orm::Error> {
        println!("Running DatabaseSeeder...");

        // Use the macro-generated Factory Builder!
        let users = User::factory(|| {
            let mut rng = rand::rng();
            let random_id: u32 = rng.random_range(1000..9999);
            User {
                id: 0, // Assigned by DB
                name: format!("Fake User {}", random_id),
                email: format!("user{}@example.com", random_id),
            }
        })
        .count(50) // Create 50 fake users!
        .create() // Saves them to the database and returns them
        .await?;

        println!("Seeded {} users successfully!", users.len());
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), rullst_orm::Error> {
    let _ = std::fs::remove_file("factories.db");
    std::fs::File::create("factories.db").unwrap();
    Orm::init("sqlite://factories.db").await?;

    Schema::create("users", |table: &mut Blueprint| {
        table.id();
        table.string("name").not_null();
        table.string("email").not_null();
    })
    .await?;

    // Execute seeders globally!
    println!("--- Starting Database Seed ---");
    Orm::seed(vec![Box::new(DatabaseSeeder)]).await?;

    // Verify it worked
    let count = User::query().get().await?.len();
    println!("Total users in database: {}", count);

    Ok(())
}
