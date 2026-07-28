use rullst_orm::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "users")]
pub struct User {
    pub id: i32,
    pub name: String,

    #[orm(has_many = "Post", foreign_key = "user_id")]
    #[sqlx(skip)]
    pub posts: Option<Vec<Post>>,
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
    let pool = Orm::pool();

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

    // Create User
    let mut user = User {
        id: 0,
        name: "Orm Dev".to_string(),
        posts: None,
    };
    user.save().await?;

    let saved_user = User::query().first().await?.unwrap();

    // Create Posts (some published, some drafts)
    let mut p1 = Post {
        id: 0,
        user_id: saved_user.id,
        title: "Rust: The Masterpiece ORM".to_string(),
        status: "published".to_string(),
    };
    let mut p2 = Post {
        id: 0,
        user_id: saved_user.id,
        title: "Vibe Coding in Rust".to_string(),
        status: "draft".to_string(),
    };
    let mut p3 = Post {
        id: 0,
        user_id: saved_user.id,
        title: "Rust: Eager Loading Tips".to_string(),
        status: "published".to_string(),
    };
    p1.save().await?;
    p2.save().await?;
    p3.save().await?;

    let all_posts = Post::query().get().await?;
    println!("All posts in DB: {:?}", all_posts);
    println!("User ID: {}", saved_user.id);

    println!(
        "🚀 Fetching user with CONSTRAINED eager loaded posts (status = 'published' AND title LIKE '%Masterpiece%'):"
    );

    // Eager load only published posts containing the word "Masterpiece"
    let users = User::query()
        .with_posts_constrained(|q| {
            q.where_eq("status", "published")
                .where_like("title", "%Masterpiece%")
        })
        .get()
        .await?;

    let fetched = &users[0];
    println!("User: {}", fetched.name);
    if let Some(ref posts) = fetched.posts {
        println!("Eager loaded {} posts:", posts.len());
        for p in posts {
            println!("  - [{}] {} (Status: {})", p.id, p.title, p.status);
        }
    } else {
        println!("No posts loaded.");
    }

    // Clean up
    let _ = std::fs::remove_file("test.db");

    Ok(())
}
