use rullst_orm::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(
    table = "users",
    before_save = "hash_password",
    after_fetch = "format_name"
)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub password: Option<String>,
}

impl User {
    // Mutator / Before Save Event
    pub async fn hash_password(&mut self) -> Result<(), rullst_orm::Error> {
        if let Some(pwd) = &self.password
            && !pwd.starts_with("hashed_")
        {
            self.password = Some(format!("hashed_{}", pwd));
            println!("[Hook: before_save] Password has been hashed!");
        }
        Ok(())
    }

    // Accessor / After Fetch Event
    pub async fn format_name(&mut self) -> Result<(), rullst_orm::Error> {
        self.name = self.name.to_uppercase();
        println!("[Hook: after_fetch] Name has been formatted to uppercase!");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), rullst_orm::Error> {
    let _ = std::fs::remove_file("hooks_test.db");
    std::fs::File::create("hooks_test.db").unwrap();
    Orm::init("sqlite://hooks_test.db").await?;
    let pool = Orm::pool()?;

    rullst_orm::_sqlx::query(
        "
        CREATE TABLE users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            password TEXT
        );
    ",
    )
    .execute(pool)
    .await?;

    // Create user
    let mut user = User {
        id: 0,
        name: "John Doe".to_string(),
        password: Some("secret123".to_string()),
    };

    println!("Saving user...");
    user.save().await?;
    println!("Saved! ID: {}, Password: {:?}", user.id, user.password);

    println!("\nFetching user...");
    let fetched = User::find(user.id).await?.unwrap();
    println!("Fetched User Name: {}", fetched.name);

    Ok(())
}
