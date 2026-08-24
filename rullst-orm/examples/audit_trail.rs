use rullst_orm::Orm;

#[derive(Clone, Debug, Default, rullst_orm::Orm, rullst_orm::FromRow)]
#[orm(table = "users", auditable)]
pub struct User {
    pub id: i32,
    pub email: String,
    pub status: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = std::fs::remove_file("test_audit.db");
    std::fs::File::create("test_audit.db").unwrap();
    Orm::init("sqlite:test_audit.db").await?;
    let pool = Orm::pool()?;

    sqlx::query(
        "CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT, email TEXT, status TEXT)",
    )
    .execute(pool)
    .await?;

    rullst_orm::audit::create_audit_table().await?;

    println!("--- Creating user ---");
    let mut user = User {
        id: 0,
        email: "john@example.com".to_string(),
        status: "active".to_string(),
    };
    user.save().await.unwrap();

    println!("--- Updating user ---");
    user.status = "banned".to_string();
    user.save().await.unwrap();

    println!("--- Querying audit logs ---");
    let audits: Vec<(String, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT event, model_type, old_values, new_values FROM rullst_audits ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await?;

    println!("Found {} audit logs:", audits.len());
    for (i, audit) in audits.iter().enumerate() {
        println!("Log {}: Event: {}, Model: {}", i + 1, audit.0, audit.1);
        println!("  Old: {:?}", audit.2);
        println!("  New: {:?}", audit.3);
    }

    Ok(())
}
