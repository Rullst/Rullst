# 🚀 Rullst Masterclass: Building a SaaS in 15 Minutes

Welcome to the definitive Rullst Masterclass! In this epic tutorial, we will take you on a journey from zero to production. You will experience the extreme productivity and raw power of the **Rullst Framework**. 

We won't just build a "Hello World". We are going to build a fully functional **Task Tracker SaaS** with database persistence, a reactive UI without writing JavaScript, an automatic Admin CMS, and then we will deploy it to the cloud.

Let's build, not suffer! 🦀

---

## 🛠️ Chapter 1: The 10-Second Scaffold

Forget spending hours configuring Webpack, Babel, or fighting with borrow checkers. Rullst gives you everything out-of-the-box.

Open your terminal and run the interactive Rullst wizard:

```bash
cargo rullst new
```

The CLI will present an elegant prompt. Make the following choices:
- **App name?** `task-master`
- **Select a Starter Blueprint?** `Blank Starter`
- **What would you like to build?** `Full-Stack Web App`
- **Enable Hot Reloading by default?** `Yes`
- **Will your project need a Database?** `Yes`
- **Select a DB Provider?** `Sqlite`

Enter the project folder and start the engine:

```bash
cd task-master
HOT_RELOAD=1 cargo run
```

Boom! Your server is running at `http://localhost:3000`. And because we enabled `HOT_RELOAD`, every time you save a `.rs` file, the server will atomically update in the background **without dropping connections or restarting**. Zero downtime! ⚡

---

## 🗄️ Chapter 2: Modeling Data & Migrations

Our SaaS needs a database table to store tasks. With Rullst, you don't write complex SQL manually. You use the built-in **Artisan CLI** to scaffold your models and migrations using pure, type-safe Rust DSL.

Run the model generator (the `-m` flag tells Rullst to generate a migration alongside the model):

```bash
cargo rullst make:model Task -m
```

This generates two files:
1. `src/models/task.rs` (Your Active Record Model)
2. `src/migrations/m[TIMESTAMP]_create_tasks_table.rs` (Your Migration)

Open the migration file and define your schema using the elegant Schema Builder:

```rust
// src/migrations/m2026...create_tasks_table.rs
use rullst_orm::schema::{Schema, Blueprint, Migration};
use rullst_orm::async_trait;

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str { "m2026_create_tasks_table" }

    async fn up(&self) -> Result<(), rullst_orm::sqlx::Error> {
        Schema::create("tasks", |table| {
            table.id();
            table.string("title").not_null();
            table.boolean("is_completed").default("false");
            table.timestamps();
        }).await?;
        Ok(())
    }

    async fn down(&self) -> Result<(), rullst_orm::sqlx::Error> {
        Schema::drop_if_exists("tasks").await?;
        Ok(())
    }
}
```

Now, let's execute the migration and create the SQLite database automatically:

```bash
cargo rullst db:migrate
```

Rullst intercepts this command, connects to the database, applies the schema, and safely exits. No external tools needed!

---

## ⚡ Chapter 3: The Magic of `html!` and HTMX

It's time to build the User Interface. Most Rust frameworks force you to use complex templating engines (like Tera or Askama) that don't understand your Rust types and break silently at runtime.

Rullst uses a **compile-time JSX-like engine** called `html!`. It's 100% type-safe, blazingly fast, and prevents XSS automatically. We will combine this with **HTMX** to get single-page application (SPA) reactivity without writing a single line of JavaScript!

Open `src/main.rs` and update the routes:

```rust
use rullst::{routes, html, Server, Router, response::{Html, IntoResponse}};
use axum::Form;
use serde::Deserialize;
// Import our newly created model
use crate::models::task::Task; 

pub mod models;
pub mod migrations;

// 1. The Dashboard View
async fn index() -> impl IntoResponse {
    let tasks = Task::all().await.unwrap_or_default();

    Html(html! {
        <html class="dark">
            <head>
                <script src="https://unpkg.com/htmx.org@1.9.10"></script>
                <script src="https://cdn.tailwindcss.com"></script>
            </head>
            <body class="bg-slate-950 text-white font-sans p-10 max-w-2xl mx-auto">
                <h1 class="text-4xl font-bold mb-8 text-emerald-400">"Task Master SaaS"</h1>
                
                // Form to create a new task. Notice the HTMX tags!
                <form 
                    hx-post="/tasks" 
                    hx-target="#task-list" 
                    hx-swap="beforeend" 
                    class="flex gap-4 mb-8"
                >
                    <input type="text" name="title" required=true class="flex-1 bg-slate-900 border border-slate-700 rounded-lg px-4 py-2" placeholder="What needs to be done?" />
                    <button type="submit" class="bg-emerald-600 hover:bg-emerald-500 font-bold py-2 px-6 rounded-lg transition-colors">"Add Task"</button>
                </form>

                <ul id="task-list" class="space-y-3">
                    { tasks.into_iter().map(|t| task_row(t)).collect::<Vec<_>>().join("") }
                </ul>
            </body>
        </html>
    })
}

// 2. A reusable component for rendering a single Task
fn task_row(task: Task) -> String {
    html! {
        <li class="p-4 bg-slate-900 rounded-lg border border-slate-800 flex justify-between items-center shadow-lg">
            <span class="font-medium text-lg">{&task.title}</span>
            <span class="text-xs text-slate-500">"Pending"</span>
        </li>
    }
}

// 3. Controller to handle form submission
#[derive(Deserialize)]
struct NewTask { title: String }

async fn create_task(Form(payload): Form<NewTask>) -> impl IntoResponse {
    let pool = rullst_orm::Orm::pool();
    
    // Insert into DB safely
    let _ = rullst_orm::sqlx::query("INSERT INTO tasks (title, is_completed, created_at, updated_at) VALUES ($1, false, datetime('now'), datetime('now'))")
        .bind(&payload.title)
        .execute(pool)
        .await;

    // Fetch the newly created task (mocking ID 999 for brevity in tutorial)
    let new_task = Task { id: 999, title: payload.title, is_completed: false };
    
    // Return ONLY the HTML for the new row. 
    // HTMX will inject this directly into the <ul>!
    Html(task_row(new_task))
}

// Register Artisan CLI interceptor
rullst::artisan!(crate::migrations::get_migrations());

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = routes![
        get("/" => index),
        post("/tasks" => create_task),
    ];

    Server::new(app).run(3000).await?;
    Ok(())
}
```

Save the file. Because Hot Reloading is active, go back to your browser at `localhost:3000`. 
Try typing a task and hitting "Add Task". **Watch it appear instantly without the page refreshing!** That's the magic of Rullst + HTMX.

---

## 🔮 Chapter 4: Rullst Nexus (The Magic Admin Panel)

Building admin panels is boring. Rullst automates this completely with the **Rullst Nexus** module. By simply implementing a trait, your model gets a gorgeous, dark-mode, AI-powered CMS panel for free.

Open `src/models/task.rs` and add the `NexusModel` implementation:

```rust
use rullst_orm::{Orm, RullstModel, sqlx::{self, FromRow}};
use rullst::{NexusModel, FieldMeta, FieldKind};

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "tasks")]
pub struct Task {
    pub id: i32,
    pub title: String,
    pub is_completed: bool,
}

// Just add this trait!
impl NexusModel for Task {
    fn nexus_table() -> &'static str { "tasks" }
    fn nexus_label() -> &'static str { "Tarefas" }
    fn nexus_icon() -> &'static str { "📋" }
    fn nexus_pk() -> &'static str { "id" }

    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta::new("id", "ID", FieldKind::Number).readonly(true),
            FieldMeta::new("title", "Título da Tarefa", FieldKind::Text),
            FieldMeta::new("is_completed", "Concluída?", FieldKind::Boolean),
        ]
    }
}
```

Now, tell your router to mount the Nexus interface in `src/main.rs`:

```rust
use rullst::Nexus;

// Inside main()...
let app = routes![
    get("/" => index),
    post("/tasks" => create_task),
].merge(Nexus::new().register::<Task>().into_router()); // Mount Nexus!
```

Open `http://localhost:3000/nexus` in your browser. 
Prepare to be amazed. You now have a complete, production-ready admin panel to create, read, update, and delete tasks, complete with an AI assistant ready to help!

---

## 🚀 Chapter 5: Deployment with Rullst Foundry

Your SaaS is ready for the world. How do you deploy a Rust application? Docker? Nginx configs? SSL certificates? Systemd services? 

Forget all that. **Rullst Foundry** is your 1-click DevOps engineer.

Initialize the Foundry configuration:

```bash
cargo rullst foundry:init
```

This creates a `Foundry.toml` file. Open it and fill in your remote server's IP address (e.g., an Ubuntu server on DigitalOcean, AWS, or Hetzner). You only need a clean Ubuntu box with SSH access.

```toml
[server]
host = "203.0.113.50"
user = "root"
```

Now, launch the deployment pipeline:

```bash
cargo rullst foundry:deploy
```

Sit back and watch the magic happen. Rullst will:
1. Compile a highly-optimized Linux production binary (`cargo build --release`).
2. SSH into your server and install dependencies (Docker, Caddy).
3. Upload your binary securely via SCP.
4. Auto-configure a Caddy HTTPS reverse-proxy and acquire free SSL certificates for your domain.
5. Setup a `systemd` persistence daemon to keep your app alive forever.

In under 2 minutes, your application is live in production, running at blazingly fast Rust speeds.

---

## 🎉 Conclusion

Congratulations! In just 15 minutes, you learned how to scaffold, model data, build reactive UI, generate an admin CMS, and deploy a production-ready application. 

**This is the Rullst promise: Rust for those who want to build, not suffer.** 🦀
