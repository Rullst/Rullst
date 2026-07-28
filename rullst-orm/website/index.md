---
layout: home

hero:
  name: "Rullst ORM"
  text: "A beautiful, type-safe, Active Record ORM for Rust."
  tagline: Built on top of SQLx, Rullst brings the delightful, fluent syntax of Active Record frameworks directly to the high-performance Rust ecosystem.
  actions:
    - theme: brand
      text: Get Started
      link: /docs/1-basics
    - theme: alt
      text: View Benchmarks
      link: /benchmarks
    - theme: alt
      text: View GitHub
      link: https://github.com/Rullst/rullst-orm

features:
  - icon: 🚀
    title: Zero-Boilerplate CRUD
    details: Insert, update, delete, and find records instantly. No more repetitive raw SQL for basic operations.
  - icon: 🔗
    title: Fluent Builder
    details: Chain `.where_eq()`, `.limit()`, and `.order_by()` effortlessly. Solve N+1 problems with robust eager loading.
  - icon: 🛡️
    title: Zero-Panic Policy
    details: Tested against rigorous standards. 100% memory safe, SQL Injection defenses built-in, and full audit logs.
  - icon: 🔒
    title: Compile-Time Privacy
    details: Automated GDPR & LGPD compliance. Protect sensitive data out-of-the-box with `#[derive(PersonalData)]` for automatic AES-256-GCM at-rest encryption and log masking.
  - icon: 🔍
    title: Miri & Kani Verified
    details: Our core logic is continuously tested against the Rust Mid-level IR (MIR) and mathematically proven via Symbolic Execution to eliminate Undefined Behavior.
  - icon: 🏗️
    title: Agnostic Schema Builder
    details: Write your migrations once using our fluent Blueprint API, and translate them seamlessly into Postgres, MySQL, or SQLite dialects on the fly.
---

<div class="glassmorphism animate-fade-in delay-2" style="padding: 2rem; margin: 4rem auto; max-width: 900px;">
  <h2 style="text-align: center; margin-bottom: 2rem; color: var(--accent-orange);">How it Works</h2>
  
  <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)); gap: 2rem;">
    <div>
      <h3 style="color: var(--accent-blue);">1. Define your Model</h3>
      <p>Just add <code>#[derive(Orm)]</code> to your standard Rust structs. No external DSLs or massive configuration files required.</p>
    </div>
    <div>
      <h3 style="color: var(--accent-blue);">2. Compile-Time Magic</h3>
      <p>The macro parses your struct fields, relationships, and metadata, generating hundreds of safe, optimized, and chainable Active Record methods instantaneously.</p>
    </div>
    <div>
      <h3 style="color: var(--accent-blue);">3. Type-Safe Queries</h3>
      <p>Query your database using fluent syntax (e.g., <code>.where_eq()</code>, <code>.limit()</code>). All parameters are bound securely via <code>sqlx</code> to completely eliminate SQL Injection risks.</p>
    </div>
  </div>
</div>

<div class="glassmorphism animate-fade-in delay-2" style="padding: 0; margin: 4rem auto; max-width: 900px; overflow: hidden; border-radius: 12px;">
  <div style="background: rgba(0, 0, 0, 0.5); padding: 0.8rem 1rem; display: flex; align-items: center; border-bottom: 1px solid var(--glass-border);">
    <span style="width: 12px; height: 12px; border-radius: 50%; background-color: #ff5f56; margin-right: 8px;"></span>
    <span style="width: 12px; height: 12px; border-radius: 50%; background-color: #ffbd2e; margin-right: 8px;"></span>
    <span style="width: 12px; height: 12px; border-radius: 50%; background-color: #27c93f; margin-right: 8px;"></span>
    <span style="margin-left: auto; margin-right: auto; font-family: var(--font-mono); font-size: 0.9rem; color: #8b949e; padding-right: 48px;">main.rs</span>
  </div>

```rust
use rullst_orm::{Orm, FromRow};

#[derive(Debug, Clone, FromRow, Orm)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
}

#[tokio::main]
async fn main() -> Result<(), rullst_orm::Error> {
    Orm::init("sqlite::memory:").await?;

    // Fluent Queries
    let active_users = User::query()
        .where_like("email", "%@example.com")
        .order_by_desc("id")
        .limit(10)
        .get()
        .await?;

    Ok(())
}
```
</div>
