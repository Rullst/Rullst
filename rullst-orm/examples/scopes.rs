use rullst_orm::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "users", global_scope = "active_only")]
pub struct User {
    pub id: i32,
    pub name: String,
    pub is_active: i32,
    pub votes: i32,
}

// -----------------------------------------
// LOCAL SCOPES & GLOBAL SCOPES
// -----------------------------------------
// The macro generated UserQueryBuilder. We can naturally add methods to it!
impl UserQueryBuilder {
    // Global Scope definition (called automatically by User::query())
    pub fn active_only(mut self) -> Self {
        self = self.where_eq("is_active", 1);
        self
    }

    // Local Scope 1
    pub fn popular(mut self) -> Self {
        self = self.where_gt("votes", 100);
        self
    }

    // Local Scope 2
    pub fn name_starts_with(mut self, prefix: &str) -> Self {
        self = self.where_like("name", format!("{}%", prefix));
        self
    }
}

#[tokio::main]
async fn main() -> Result<(), rullst_orm::Error> {
    let _ = std::fs::remove_file("scopes_test.db");
    std::fs::File::create("scopes_test.db").unwrap();
    Orm::init("sqlite://scopes_test.db").await?;
    let pool = Orm::pool()?;

    rullst_orm::_sqlx::query(
        "
        CREATE TABLE users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            is_active INTEGER DEFAULT 1,
            votes INTEGER DEFAULT 0
        );
    ",
    )
    .execute(pool)
    .await?;

    // Insert dummy data
    rullst_orm::_sqlx::query("INSERT INTO users (name, is_active, votes) VALUES ('Alice', 1, 150)")
        .execute(pool)
        .await?;
    rullst_orm::_sqlx::query("INSERT INTO users (name, is_active, votes) VALUES ('Bob', 0, 200)")
        .execute(pool)
        .await?;
    rullst_orm::_sqlx::query(
        "INSERT INTO users (name, is_active, votes) VALUES ('Charlie', 1, 50)",
    )
    .execute(pool)
    .await?;
    rullst_orm::_sqlx::query(
        "INSERT INTO users (name, is_active, votes) VALUES ('Amanda', 1, 120)",
    )
    .execute(pool)
    .await?;

    // 1. Query with Global Scope (Automatically filters out Bob because is_active = 0)
    let active_users = User::query().get().await?;
    println!("--- Active Users (Global Scope) ---");
    for user in active_users {
        println!("- {} (Votes: {})", user.name, user.votes);
    }

    // 2. Query with Global Scope + Local Scopes (Popular + Name starts with A)
    let popular_a_users = User::query().popular().name_starts_with("A").get().await?;
    println!("\n--- Popular Active Users starting with 'A' (Global + Local Scopes) ---");
    for user in popular_a_users {
        println!("- {} (Votes: {})", user.name, user.votes);
    }

    // 3. Query bypassing Global Scopes (Using raw builder)
    let all_users = UserQueryBuilder::new().get().await?;
    println!("\n--- All Users (Bypassing Global Scopes) ---");
    for user in all_users {
        println!(
            "- {} (Active: {}, Votes: {})",
            user.name, user.is_active, user.votes
        );
    }

    Ok(())
}
