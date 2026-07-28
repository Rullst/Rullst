use rullst_orm::Orm;

#[derive(Clone, Debug, Default, rullst_orm::Orm, rullst_orm::FromRow)]
#[orm(table = "documents", searchable)]
pub struct Document {
    pub id: i32,
    pub title: String,
    pub body: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = std::fs::remove_file("test_scout.db");
    std::fs::File::create("test_scout.db").unwrap();
    Orm::init("sqlite:test_scout.db").await?;
    let pool = Orm::pool();

    sqlx::query(
        "CREATE TABLE documents (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT, body TEXT)",
    )
    .execute(pool)
    .await?;

    let mut doc1 = Document {
        id: 0,
        title: "Rust ORM Guide".to_string(),
        body: "Learn how to build a scalable backend.".to_string(),
    };
    doc1.save().await.unwrap();

    let mut doc2 = Document {
        id: 0,
        title: "Full-Text Search".to_string(),
        body: "Scout makes searching models incredibly easy in Rust.".to_string(),
    };
    doc2.save().await.unwrap();

    println!("--- Searching for 'ORM' ---");
    let results = Document::search("ORM").await.get().await?;
    for doc in results {
        println!("Match: {} (ID: {})", doc.title, doc.id);
    }

    println!("\n--- Searching for 'Rust' ---");
    let results = Document::search("Rust").await.get().await?;
    for doc in results {
        println!("Match: {} (ID: {})", doc.title, doc.id);
    }

    Ok(())
}
