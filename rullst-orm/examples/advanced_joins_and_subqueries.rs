use rullst_orm::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "users")]
pub struct User {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "posts")]
pub struct Post {
    pub id: i32,
    pub user_id: i32,
    pub title: String,
    pub status: String,
}

#[tokio::main]
async fn main() -> Result<(), rullst_orm::Error> {
    let _ = std::fs::remove_file("test.db");
    std::fs::File::create("test.db").unwrap();
    Orm::init("sqlite://test.db").await?;
    let pool = Orm::pool()?;

    rullst_orm::_sqlx::query(
        "
        CREATE TABLE users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL
        );
    ",
    )
    .execute(pool)
    .await?;

    rullst_orm::_sqlx::query(
        "
        CREATE TABLE posts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            title TEXT NOT NULL,
            status TEXT NOT NULL
        );
    ",
    )
    .execute(pool)
    .await?;

    // Create users
    let mut u1 = User {
        id: 0,
        name: "Alice".to_string(),
    };
    let mut u2 = User {
        id: 0,
        name: "Bob".to_string(),
    };
    u1.save().await?;
    u2.save().await?;

    let saved_u1 = User::query().first().await?.unwrap();
    let saved_u2 = User::query()
        .where_eq("name", "Bob")
        .first()
        .await?
        .unwrap();

    // Create posts
    let mut p1 = Post {
        id: 0,
        user_id: saved_u1.id,
        title: "Rust Masterpiece".to_string(),
        status: "published".to_string(),
    };
    let mut p2 = Post {
        id: 0,
        user_id: saved_u1.id,
        title: "Vibe Coding".to_string(),
        status: "draft".to_string(),
    };
    let mut p3 = Post {
        id: 0,
        user_id: saved_u2.id,
        title: "SeaORM Guide".to_string(),
        status: "published".to_string(),
    };
    p1.save().await?;
    p2.save().await?;
    p3.save().await?;

    // 1. Test where_exists Subquery
    // Find users who have at least one PUBLISHED post
    println!("🚀 Querying users using 'where_exists' subquery...");
    let active_users = User::query()
        .where_exists(
            Post::query()
                .select_cols(&[PostColumn::Id])
                .where_column("posts.user_id", "users.id")
                .where_eq("posts.status", "published"),
        )
        .get()
        .await?;

    println!("Active Users (with published posts):");
    for u in &active_users {
        println!("  - [{}] {}", u.id, u.name);
    }

    // 2. Test join_constrained
    // Join posts table with multiple ON constraints
    println!("\n🚀 Querying posts joining with users using 'join_constrained'...");
    let posts_with_users = Post::query()
        .select_raw("posts.*, users.name as author_name")
        .join_constrained("users", |join| {
            join.on("posts.user_id", "=", "users.id")
                .on_eq("users.name", "Alice")
        })
        .where_eq("posts.status", "published")
        .get()
        .await?;

    // Since FromRow doesn't map virtual select_raw author_name directly into Post struct without extra fields,
    // we can fetch them or prove it compiles and runs without sqlite schema exceptions.
    println!("Returned posts count for Alice: {}", posts_with_users.len());
    for p in &posts_with_users {
        println!("  - Post: {} (Author ID: {})", p.title, p.user_id);
    }

    // Clean up
    let _ = std::fs::remove_file("test.db");

    Ok(())
}
