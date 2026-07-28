use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{FromRow, Orm};

// When we derive Orm, it will generate a `UserColumn` enum automatically
// because our struct is named `User`!
#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "users")]
pub struct User {
    pub id: i32,
    pub full_name: String,
    pub age: i32,
}

#[tokio::main]
async fn main() -> Result<(), rullst_orm::Error> {
    // 1. Initialize DB & Schema
    let _ = std::fs::remove_file("compile_time_safety.db");
    std::fs::File::create("compile_time_safety.db").unwrap();
    Orm::init("sqlite://compile_time_safety.db").await?;

    Schema::create("users", |table: &mut Blueprint| {
        table.id();
        table.string("full_name").not_null();
        table.integer("age").not_null();
    })
    .await?;

    // 2. Insert Test Data
    let mut u1 = User {
        id: 0,
        full_name: "Alice Smith".to_string(),
        age: 25,
    };
    u1.save().await?;
    let mut u2 = User {
        id: 0,
        full_name: "Bob Jones".to_string(),
        age: 30,
    };
    u2.save().await?;

    // --------------------------------------------------------------------------------
    // COMPILE-TIME SAFE QUERIES!
    // Instead of using magic strings like `.where_eq("full_name", "Alice")`
    // We use the generated `UserColumn` enum! No more typos!
    // --------------------------------------------------------------------------------
    println!("--- Testing Compile-Time Safety ---");

    // 1. Select specific columns
    // We want only the Id and FullName. Age will be mapped as 0/default by sqlx
    let users = User::query()
        .select_cols(&[UserColumn::Id, UserColumn::FullName, UserColumn::Age])
        .where_col(UserColumn::Age, 25)
        .order_by_desc_col(UserColumn::Id)
        .get()
        .await?;

    for u in users {
        println!("Found user: {} (ID: {})", u.full_name, u.id);
    }

    Ok(())
}
