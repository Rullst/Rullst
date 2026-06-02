<p align="center">
  <img src="https://venelouis.github.io/Rullst/Rullst.png" alt="Rullst Logo" width="400">
</p>

# Rullst - 📜🦀🌐🤖🚀
### *"Rust for those who want to build, not suffer."*

> 📖 **[See all the changes in our Changelog!](https://github.com/venelouis/Rullst/blob/main/CHANGELOG.md)**  
> 📚 **[Read the Official Documentation!](https://venelouis.github.io/Rullst/)**  
> 📦 **[View on Crates.io!](https://crates.io/crates/rullst)**



<p align="center">
  <img src="https://img.shields.io/crates/v/rullst?style=for-the-badge&color=fc8d62&logo=rust" alt="Crates.io">
  <img src="https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge" alt="License: MIT">
  <img src="https://img.shields.io/docsrs/rullst?style=for-the-badge&color=4d76ae" alt="Docs">
  <img src="https://img.shields.io/crates/d/rullst?style=for-the-badge&color=8da0cb" alt="Downloads">
</p>

**Rullst** (Rust + Fullstack) is an opinionated, developer-first full-stack web framework for Rust, obsessively designed for **Emotional Productivity**. 

It was created to solve the biggest problem in the Rust web ecosystem: the high barrier of entry that turns web programming into a PhD research on compiler design. We believe you should spend your energy building your business, not fighting borrow checkers and manual routing setups.

---

## 💡 The Rullst Manifesto

> *"Most Rust frameworks treat the web developer like a compiler engineer. Rullst treats the developer like someone who wants to build awesome products at lightning speed."*

In the current ecosystem, to write a simple CRUD, you are forced to glue dozens of crates together, manually map nested routing trees, write verbose ORMs requiring multiple structs, and continuously clone variables inside dynamic HTML templates just to satisfy the borrow checker.

Rullst redefines this experience. We offer an integrated, cohesive developer experience that brings the sweetness and iteration speed of **Laravel and Next.js** together with the Formula 1 performance and military-grade safety of **Rust, Axum, and Hyper**:

* **No More Frankenstein setups:** A single cohesive framework managing your server (Axum), your database (`rullst-orm`), and your HTML rendering.
* **No More Borrow Checker fights in UI:** Our compile-time JSX-like `html!` macro processes pure elements on the server (SSR). It generates optimized string-builders directly at compile time. It's blazing fast, safe, and SEO-friendly by default.
* **First-Class Active Record ORM:** Native integration with your **`rullst-orm`** package. Interacting with databases is as intuitive as `user.save()`.
* **AI-Native Engineering & AI-Friendly:** Designed from the ground up for modern pair-programming. Strict type-safety, zero dynamic runtime magic, automatic `.ai-rules` scaffolding, and structured schemas prevent AI agent hallucinations and allow instant compiler self-correction.

---

## 🏆 Everything You Need, Built-In

Rullst ships with **10 completed milestones** covering every layer of modern web development:

| Category | Features |
|---|---|
| 🛠️ **CLI & DX** | `cargo rullst new` wizard (interactive blueprints: Blank, LMS, SaaS, Blog, ERP, Uptime), `make:controller`, `make:model -m`, `make:middleware`, `make:worker`, `generate:openapi`, `cargo rullst upgrade` (self-healing) |
| 🗄️ **Database** | Active Record ORM, Migrations (`db:migrate`, `db:rollback`, `db:status`), Seeders & Factories, HasMany / BelongsTo / BelongsToMany, Eager Loading |
| 🔒 **Auth & Security** | Argon2 hashing, JWT & Cookie sessions, CSRF protection, Social OAuth (Google, GitHub, Facebook, Twitter via `rullst-connect`), `cargo rullst auth` scaffolding |
| ⚡ **Frontend** | HTMX first-class support, TailwindCSS auto-integration, partial template rendering, **Rullst Live** (Phoenix LiveView-inspired server-driven UI), **Wasm Islands** (`#[client_component]`) |
| 📦 **Production** | Queue (SQLite/Redis), Cache (Memory/Redis), Task Scheduler (Cron), Docker multi-stage builds, **Rullst Horizon** dashboard |
| 🏢 **Enterprise** | Declarative Validation, Mailer (SMTP/Resend/SendGrid), Storage (Local/S3/R2), WebSockets, Multi-Tenancy, Feature Flags, E2E Testing |
| 🚀 **Unfair Advantage** | **AI Core** (`rullst::ai` — OpenAI/Gemini/Anthropic/Ollama + RAG), **Rullst Studio** (visual DB GUI), **Self-Healing Error Console** (AI auto-fix), **Hot Reloading via `dylib`** |
| 🌍 **Edge & Data** | Edge Runtime (Cloudflare Workers, Fastly, AWS Lambda@Edge), Zero-Config Distributed SQLite Replication (Turso/D1), Autonomous Upgrades |
| 🆓 **Free Enterprise** | **Rullst Nexus** (visual auto-CMS & AI Chat assistant), **Rullst Capital** (Stripe/LemonSqueezy subscriptions boilerplate), **Dual-Engine Frontend** (Hyper Desktop + Omni Multi-Platform Signal Apps), **Rullst Shield** (Wasm WAF & PII masking), **Rullst Foundry** (1-click Cloud Devops Deploy) |
| ⚡ **Linker Hacking** | **Mold/Cranelift Deep Integration** (sub-100ms incremental hot loops), compiler-driven auto-link optimization |

---

## 🎨 The "Hello World" That Conquers at First Sight

This is a complete, fully operational web server with type-safe routing, compile-time HTML rendering, and automatic XSS escaping. It is exactly **20 lines of code**:

```rust
use rullst::{html, routes, Server, Router, response::{Html, IntoResponse}};

async fn hello() -> impl IntoResponse {
    Html(html! {
        <main style="display: grid; place-items: center; height: 100vh; background: #090d16; color: #fff; font-family: system-ui;">
            <div style="text-align: center;">
                <h1 style="font-size: 4rem; margin: 0; background: linear-gradient(135deg, #00f2fe, #4facfe); -webkit-background-clip: text; -webkit-text-fill-color: transparent;">
                    "Hello, World!"
                </h1>
                <p style="color: #64748b; font-size: 1.25rem; margin-top: 1rem;">
                    "Written in Rust. Rendered in microseconds. Safe by default."
                </p>
            </div>
        </main>
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Server::new(routes![get("/" => hello)])
        .run(3000)
        .await?;
    Ok(())
}
```

---

## ⚡ Get Started in 10 Seconds

Scaffold a fully operational application with our interactive CLI wizard!

```bash
# 1. Run the interactive CLI scaffolding tool
cargo rullst new

# The wizard will prompt you:
# 🚀 App name? (no spaces allowed) -> my-app
# 🧭 Select a Starter Blueprint -> 
#     * Blank Starter (Minimal template with HTMX reactive counter)
#     * LMS Platform (Courses, lessons, video player, HTMX integration)
#     * SaaS App Starter (Authentication + Stripe payments billing template)
#     * Blog / Press (Static site generator pre-wired with Nexus CMS)
#     * ERP Pocket (Inventory, stock management, orders tracker, auto-CMS)
#     * Uptime Monitor (Ping dashboard, background status checker, glassmorphism)
# 
# (If Blank Starter is selected, it will customize further):
#   🏗️ What would you like to build? -> Full-Stack Web App / Headless REST API
#   🔥 Enable Hot Reloading by default? -> Yes / No
#   🗄️ Will your project need a Database? -> Yes / No
#   💾 Select a DB Provider -> Sqlite / Postgres / MySQL/MariaDB

# 2. Enter the project folder
cd my-app

# 3. Start your high-performance full-stack app immediately!
cargo run

# 🔥 Or enable instant Hot Reloading (no server restart!):
HOT_RELOAD=1 cargo run
```

---

## 🛠️ The Full-Stack Active Record Experience

When your application grows, Rullst scales with you using Active Record:

```rust
use rullst::{html, routes, Server, Router, response::{Html, IntoResponse}};
use rullst::{Orm, RullstModel, db::{sqlx, FromRow}};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "users")]
pub struct User {
    pub id: i32,
    pub name: String,
}

async fn home() -> impl IntoResponse {
    // Elegant, type-safe data fetching
    let users = User::all().await.unwrap();

    Html(html! {
        <div style="background: #0f172a; color: #fff; padding: 5rem; text-align: center; font-family: sans-serif;">
            <h1>"Total Active Users: " {users.len()}</h1>
        </div>
    })
}

// 1. Declare the artisan macro here to intercept CLI arguments for migrations
rullst::artisan!();

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 2. The artisan! macro automatically intercepts `db:*` commands and exits early.
    // If it's a normal run, it continues server execution here.

    let router = routes![
        get("/" => home),
    ];

    Server::new(router)
        .run(3000)
        .await?;

    Ok(())
}
```

---

## 🗄️ Database Migrations (Artisan CLI)

Rullst includes an embedded, high-performance migration runner. You don't need external binaries. The framework ships with a CLI tool that parses pure Rust closures to construct your schema safely.

```bash
# Scaffold a new migration using pure Rust DSL
cargo rullst make:migration create_users_table

# Run all pending migrations against your database
cargo rullst db:migrate

# Rollback the last batch of migrations
cargo rullst db:rollback
```

Under the hood, these commands are intercepted by the `rullst::artisan!()` macro, guaranteeing the server never starts when you only want to migrate your database.

---

## 🛡️ Self-Healing Upgrades

Afraid of breaking changes when upgrading the framework? Don't be. Rullst was built with a "Self-Healing Upgrades" philosophy. 

When a new version of Rullst introduces API changes, we never break your code immediately. Instead, we use `#[deprecated]` warnings. You can update your entire application automatically using our CLI:

```bash
cargo rullst upgrade
```

This command will safely update the Rullst dependency and use Rust's powerful `cargo fix` refactoring tools to automatically rewrite your code to match the new API signatures. Stress-free upgrades, forever.

---

## 🔥 Hot Reloading (Zero Downtime Dev Loop)

Rullst supports **Hot Reloading via Dynamic Linking** — change your routes, handlers, and templates, and see the changes reflected **instantly** without restarting the server or losing connections:

```bash
# Start your app in hot-reload mode
HOT_RELOAD=1 cargo run

# ⚡ Edit any handler in src/ → Rullst detects the change
# 🔄 Background recompilation of the cdylib
# 🚀 Router hot-swapped atomically — zero downtime!
```

Under the hood, Rullst compiles your routes as a dynamic library (`cdylib`), loads it via `libloading`, and uses a `notify` file-watcher to detect changes and trigger background rebuilds. The router is swapped atomically via `Arc<RwLock<Router>>` — the HTTP server never restarts and TCP connections are never dropped.

---

## 🎯 Architecture under the hood

Rullst is structured as a modular monorepo Cargo Workspace to optimize compile times:

1. **`rullst` (Core Crate):** Wraps and configures Axum, handles life-cycle DB injection, and exposes response types. Ships with production utilities (Queue, Cache, Scheduler), enterprise features (Validation, Mailer, Storage, WebSockets, Horizon), AI-Native core (`rullst::ai`), Rullst Live (server-driven UI), Wasm Islands, and Hot Reloading via dynamic linking.
2. **`rullst-macros` (Compiler-Engine):** Procedural JSX-like compiler that outputs safe memory-buffer string extensions at compile time.
3. **`cargo-rullst` (CLI Scaffold):** Scaffolds clean, isolated local-linked workspaces that compile out-of-the-box.

For detailed technical conventions, directory structures, and framework APIs, refer to our [Official Specification (SST)](./docs/spec.md).

---

## 📝 License

Distributed under the MIT License. See `LICENSE` for more details.
