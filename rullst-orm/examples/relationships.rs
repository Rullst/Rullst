use rullst_orm::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "users")]
pub struct User {
    pub id: i32,
    pub name: String,

    // Virtual relationships field
    #[orm(has_many = "Post", foreign_key = "user_id")]
    #[sqlx(skip)]
    pub posts: Option<Vec<Post>>,

    // Enables soft deletes automatically
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "posts")]
pub struct Post {
    pub id: i32,
    pub user_id: i32,
    pub title: String,

    #[orm(belongs_to = "User", foreign_key = "user_id")]
    #[sqlx(skip)]
    pub author: Option<User>,
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
            name TEXT NOT NULL,
            deleted_at TEXT
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
            title TEXT NOT NULL
        );
    ",
    )
    .execute(pool)
    .await?;

    // Insert user
    let mut user = User {
        id: 0,
        name: "Eager User".to_string(),
        posts: None,
        deleted_at: None,
    };
    user.save().await?;

    // Insert posts
    let saved_user = User::query().first().await?.unwrap();
    let mut post1 = Post {
        id: 0,
        user_id: saved_user.id,
        title: "Post 1".to_string(),
        author: None,
    };
    let mut post2 = Post {
        id: 0,
        user_id: saved_user.id,
        title: "Post 2".to_string(),
        author: None,
    };
    post1.save().await?;
    post2.save().await?;

    // Fetch Eagerly!
    let users = User::query().with_posts().get().await?;

    let fetched_user = users.first().unwrap();
    println!("Fetched user: {}", fetched_user.name);

    let posts = fetched_user.posts.as_ref().unwrap();
    println!("Eager loaded {} posts for this user!", posts.len());
    for post in posts {
        println!(" - {}", post.title);
    }

    Ok(())
}
