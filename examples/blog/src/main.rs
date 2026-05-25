use rullst::{html, routes, Server, Router, response::{Html, IntoResponse, Redirect}};
use rust_eloquent::{Eloquent, EloquentModel, sqlx::{self, FromRow}};
use axum::Form;

// 1. Define the Post database model
#[derive(Debug, Clone, FromRow, rust_eloquent::Eloquent)]
#[eloquent(table = "posts")]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub body: String,
}

#[derive(serde::Deserialize)]
pub struct CreatePostForm {
    pub title: String,
    pub body: String,
}

// 2. Render the Blog homepage with the posting form and list of posts
async fn index() -> impl IntoResponse {
    let posts = Post::all().await.unwrap_or_default();

    Html(html! {
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <title>"Rullst Dev Blog - Built in Rust"</title>
                <style>
                    "
                    body {
                        background: #0b0f19;
                        color: #f1f5f9;
                        font-family: system-ui, -apple-system, sans-serif;
                        margin: 0;
                        padding: 0;
                        display: flex;
                        justify-content: center;
                    }
                    .container {
                        width: 100%;
                        max-width: 800px;
                        padding: 3rem 1.5rem;
                    }
                    header {
                        text-align: center;
                        margin-bottom: 3.5rem;
                    }
                    h1 {
                        font-size: 3rem;
                        margin: 0 0 0.5rem 0;
                        background: linear-gradient(135deg, #38bdf8, #818cf8);
                        -webkit-background-clip: text;
                        -webkit-text-fill-color: transparent;
                        font-weight: 800;
                    }
                    p.subtitle {
                        color: #64748b;
                        font-size: 1.2rem;
                        margin: 0;
                    }
                    .card {
                        background: #111827;
                        border: 1px solid #1f2937;
                        border-radius: 0.75rem;
                        padding: 2rem;
                        margin-bottom: 2rem;
                        box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
                    }
                    .form-title {
                        font-size: 1.5rem;
                        margin-top: 0;
                        margin-bottom: 1.5rem;
                        font-weight: 600;
                        color: #38bdf8;
                    }
                    .form-group {
                        margin-bottom: 1.25rem;
                    }
                    label {
                        display: block;
                        font-size: 0.875rem;
                        color: #94a3b8;
                        margin-bottom: 0.5rem;
                        font-weight: 500;
                    }
                    input[type='text'], textarea {
                        width: 100%;
                        box-sizing: border-box;
                        background: #1f2937;
                        border: 1px solid #374151;
                        border-radius: 0.5rem;
                        padding: 0.75rem 1rem;
                        color: #fff;
                        font-size: 1rem;
                        font-family: inherit;
                        transition: border-color 0.2s, box-shadow 0.2s;
                    }
                    input[type='text']:focus, textarea:focus {
                        outline: none;
                        border-color: #6366f1;
                        box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.2);
                    }
                    button {
                        background: linear-gradient(135deg, #6366f1, #4f46e5);
                        color: #fff;
                        border: none;
                        border-radius: 0.5rem;
                        padding: 0.75rem 1.5rem;
                        font-size: 1rem;
                        font-weight: 600;
                        cursor: pointer;
                        transition: transform 0.1s, opacity 0.2s;
                    }
                    button:hover {
                        opacity: 0.9;
                        transform: translateY(-1px);
                    }
                    button:active {
                        transform: translateY(0);
                    }
                    .post-list-title {
                        font-size: 1.75rem;
                        margin-top: 3rem;
                        margin-bottom: 1.5rem;
                        border-bottom: 1px solid #1f2937;
                        padding-bottom: 0.5rem;
                        font-weight: 700;
                    }
                    .post-card {
                        background: #111827;
                        border-left: 4px solid #6366f1;
                        border-radius: 0.5rem;
                        padding: 1.5rem;
                        margin-bottom: 1.5rem;
                        transition: transform 0.2s, box-shadow 0.2s;
                    }
                    .post-card:hover {
                        transform: translateY(-2px);
                        box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.3);
                    }
                    .post-title {
                        font-size: 1.35rem;
                        margin: 0 0 0.5rem 0;
                        font-weight: 600;
                    }
                    .post-body {
                        color: #cbd5e1;
                        line-height: 1.6;
                        margin: 0;
                        white-space: pre-wrap;
                    }
                    .empty-state {
                        text-align: center;
                        color: #475569;
                        padding: 3rem;
                        font-style: italic;
                    }
                    "
                </style>
            </head>
            <body>
                <div class="container">
                    <header>
                        <h1>"Rullst Dev Blog"</h1>
                        <p class="subtitle">"Sleek full-stack Rust blog powered by Rullst &amp; Active Record"</p>
                    </header>

                    <div class="card">
                        <div class="form-title">"Create New Post"</div>
                        <form method="post" action="/posts">
                            <div class="form-group">
                                <label htmlFor="title">"Post Title"</label>
                                <input type="text" id="title" name="title" placeholder="What's on your mind?" required="required" />
                            </div>
                            <div class="form-group">
                                <label htmlFor="body">"Content"</label>
                                <textarea id="body" name="body" rows="5" placeholder="Share your Rust thoughts here..." required="required"></textarea>
                            </div>
                            <button type="submit">"Publish Post"</button>
                        </form>
                    </div>

                    <div class="post-list-title">"Published Stories"</div>
                    <div>
                        {
                            if posts.is_empty() {
                                html! {
                                    <div class="empty-state">
                                        "No posts published yet. Be the first to share a story!"
                                    </div>
                                }
                            } else {
                                // Compile-time JSX lists expansion!
                                // Rullst handles vectors of strings seamlessly
                                let mut post_list = String::new();
                                for post in posts.iter().rev() {
                                    let content = html! {
                                        <div class="post-card">
                                            <h3 class="post-title">{post.title}</h3>
                                            <p class="post-body">{post.body}</p>
                                        </div>
                                    };
                                    post_list.push_str(&content);
                                }
                                html! {
                                    { rullst::html::RawHtml(post_list) }
                                }
                            }
                        }
                    </div>
                </div>
            </body>
        </html>
    })
}

// 3. Handle the form submission, store the post using Active Record, and redirect
async fn store(Form(form): Form<CreatePostForm>) -> Redirect {
    if !form.title.trim().is_empty() && !form.body.trim().is_empty() {
        let mut post = Post {
            id: 0,
            title: form.title,
            body: form.body,
        };
        // Saves to SQLite magically using rust-eloquent INSERT!
        let _ = post.save().await;
    }
    Redirect::to("/")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 4. Initialize in-memory SQLite database
    Eloquent::init("sqlite::memory:").await?;

    // Create table schema
    let pool = Eloquent::pool();
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS posts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            body TEXT NOT NULL
        )"
    )
    .execute(pool)
    .await?;

    // Populate with a seed post!
    let mut initial_post = Post {
        id: 0,
        title: "Why Rullst is a Game Changer".to_string(),
        body: "Because it lets us build high-performance web applications with Rust without losing the emotional productivity and joy that Rails or Laravel gave us. No borrow-checker fights in templates, no convoluted routing, just pure web building fun!".to_string(),
    };
    initial_post.save().await?;

    // 5. Centralized Laravel-style routing in action!
    let router = routes![
        get("/" => index),
        post("/posts" => store),
    ];

    Server::new(router)
        .run(3000)
        .await?;

    Ok(())
}
