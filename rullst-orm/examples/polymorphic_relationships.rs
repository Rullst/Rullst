use rullst_orm::schema::Schema;
use rullst_orm::{FromRow, Orm, RullstModel};

// The Comment model represents a polymorphic child.
// It can belong to either a Post or a Video.
#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "comments")]
pub struct Comment {
    pub id: i32,
    pub body: String,
    pub commentable_id: i32,
    pub commentable_type: String,
}

// A Post can have many Comments
#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "posts")]
pub struct Post {
    pub id: i32,
    pub title: String,

    // The polymorphic relationship!
    #[orm(morph_many = "Comment", name = "commentable")]
    #[sqlx(skip)]
    pub comments: Option<Vec<Comment>>,
}

// A Video can ALSO have many Comments!
#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "videos")]
pub struct Video {
    pub id: i32,
    pub url: String,

    #[orm(morph_many = "Comment", name = "commentable")]
    #[sqlx(skip)]
    pub comments: Option<Vec<Comment>>,
}

#[tokio::main]
async fn main() -> Result<(), rullst_orm::Error> {
    let _ = std::fs::remove_file("polymorphic.db");
    std::fs::File::create("polymorphic.db").unwrap();
    Orm::init("sqlite://polymorphic.db").await?;

    Schema::create("posts", |table| {
        table.id();
        table.string("title").not_null();
    })
    .await?;

    Schema::create("videos", |table| {
        table.id();
        table.string("url").not_null();
    })
    .await?;

    Schema::create("comments", |table| {
        table.id();
        table.string("body").not_null();
        table.integer("commentable_id").not_null();
        table.string("commentable_type").not_null();
    })
    .await?;

    // Create a Post and a Video
    let mut post = Post {
        id: 0,
        title: "Rust ORM Guide".to_string(),
        comments: None,
    };
    post.save().await?;

    let mut video = Video {
        id: 0,
        url: "https://youtube.com/rust".to_string(),
        comments: None,
    };
    video.save().await?;

    // Add comments to the Post
    let mut c1 = Comment {
        id: 0,
        body: "Great post!".to_string(),
        commentable_id: post.id,
        commentable_type: Post::table_name().to_string(),
    };
    c1.save().await?;

    // Add comments to the Video
    let mut c2 = Comment {
        id: 0,
        body: "Awesome video!".to_string(),
        commentable_id: video.id,
        commentable_type: Video::table_name().to_string(),
    };
    c2.save().await?;

    // Test polymorphic eager loading!
    let fetched_post = Post::query().with_comments().first().await?.unwrap();
    println!("Post: {}", fetched_post.title);
    for comment in fetched_post.comments.unwrap() {
        println!(" - Comment: {}", comment.body);
    }

    let fetched_video = Video::query().with_comments().first().await?.unwrap();
    println!("Video: {}", fetched_video.url);
    for comment in fetched_video.comments.unwrap() {
        println!(" - Comment: {}", comment.body);
    }

    Ok(())
}
