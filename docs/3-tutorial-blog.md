# Tutorial: Building a Blog with Rullst Framework

In this tutorial, we will use the **Rullst Framework** to create the backend and the Server-Side Rendered (SSR) frontend of a full Blog!

This guide assumes you already have the Rullst CLI installed (`cargo install cargo-rullst`).

## 1. Creating the Project

The first step is to generate the entire Rullst project structure:

```bash
cargo rullst new my_blog
cd my_blog
```

The above command will create the `src/controllers`, `src/models`, `src/pages` folders, and set up your `Rullst.toml` file ready for development with an in-memory SQLite database.

## 2. Creating the Database Model and Table (Migration)

In a blog, we'll need to store posts. We'll use the Rullst Artisan Engine to create an Active Record Model and a Migration simultaneously:

```bash
cargo rullst make:model Post --migration
```

This generates two files. The first is the Migration `src/migrations/m2026_create_posts.rs`. Open it and add the columns for our posts table:

```rust
// In src/migrations/...create_posts.rs
async fn up(&self) -> Result<(), rust_eloquent::sqlx::Error> {
    Schema::create("posts", |table| {
        table.id();
        table.string("title");
        table.string("content");
        table.timestamps();
    }).await
}
```

The second file is the Model `src/models/post.rs`. It uses the `#[derive(Eloquent)]` macro. In it, we need to reflect the properties:

```rust
#[derive(Debug, Clone, FromRow, rust_eloquent::Eloquent)]
#[eloquent(table = "posts")]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub content: String,
}
```

To apply the table to the SQLite database immediately:

```bash
cargo run --bin my_blog -- db:migrate
```

## 3. Creating the Controller

With the database ready, let's create the HTTP routes and logic to view the posts. Let's create a Posts Controller:

```bash
cargo rullst make:controller Posts
```

The `src/controllers/posts_controller.rs` file will be generated with HTTP functions `index`, `show`, `store` already configured.

Let's edit the `index()` function so it fetches all posts from the database and displays them using Rullst's `html!` macro (SSR):

```rust
use crate::models::post::Post;
use rullst::{html, response::{Html, IntoResponse}};

pub async fn index() -> impl IntoResponse {
    // 1. Fetch all Posts from the DB using rust-eloquent
    let posts = Post::all().await.unwrap_or_default();
    
    // 2. Render a functional HTML String
    let mut posts_html = String::new();
    for p in posts {
        posts_html.push_str(&html! {
            <article class="p-4 border rounded mb-4">
                <h2 class="text-xl font-bold">{p.title}</h2>
                <p class="text-gray-600">{p.content}</p>
            </article>
        });
    }

    // 3. Return the response injecting the "raw" String
    Html(html! {
        <div class="max-w-2xl mx-auto py-10">
            <h1 class="text-3xl font-bold mb-6">"My Rust Blog"</h1>
            { rullst::html::RawHtml(posts_html) }
        </div>
    })
}
```

## 4. Routing

Finally, we need to tell Rullst that the main HTTP Route (`/`) should point to our `index` function in the `posts_controller`.

Open the `src/main.rs` file, find the `routes!` declaration and change it:

```rust
use crate::controllers::posts_controller;

let router = routes![
    get("/" => posts_controller::index),
];
```

## 5. Running!

Done! Just run the web server and open your browser:

```bash
cargo run
```

Your lightning-fast blog is running! Try adding manual records via `cargo run --bin my_blog -- studio` (The native Rullst Studio) and see them appear in milliseconds on your page.
