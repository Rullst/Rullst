# 4. Migrations & Schema Builder

Rullst ORM includes a database-agnostic schema builder called `Blueprint`. It translates your column definitions into the appropriate SQL dialect (Postgres, MySQL, or SQLite) dynamically at runtime.

## Defining Schemas

Use `Schema::create` to map a new table. Inside the closure, the `Blueprint` allows you to declare typed columns fluently.

```rust
use rullst_orm::schema::Schema;

#[tokio::main]
async fn main() -> Result<(), rullst_orm::Error> {
    rullst_orm::Orm::init("sqlite::memory:").await?;

    Schema::create("users", |bp| {
        bp.id(); // Creates an auto-incrementing integer primary key named `id`
        bp.string("name").not_null();
        bp.string("email").unique();
        bp.boolean("is_active").default(rullst_orm::schema::ColumnDefault::Integer(1));
        
        // Adds `created_at` and `updated_at` DATETIME columns
        bp.timestamps(); 
        
        // Adds a nullable `deleted_at` column for soft deleting
        bp.soft_deletes(); 
    }).await?;

    Ok(())
}
```

### Dropping Tables

You can safely drop a table or manage its lifecycle securely without resorting to raw DDL strings:

```rust
Schema::drop_if_exists("users").await?;
```

---

## 🐘 The Artisan CLI

For production environments, hardcoding `Schema::create` across your codebase isn't ideal. Rullst provides an integrated `run_artisan` command handler.

It intercepts standard command-line arguments to scaffold and execute database migrations exactly like PHP's Artisan or Rails' Active Record Migrations.

### Setup

```rust
use rullst_orm::schema::run_artisan;

#[tokio::main]
async fn main() {
    // 1. Init your database
    rullst_orm::Orm::init("postgres://localhost/db").await.unwrap();

    // 2. Delegate execution to Artisan, passing any seeders if desired.
    run_artisan(std::env::args().collect(), vec![]).await;
}
```

### Using Artisan

Once you compile your binary (e.g., `cargo run`), you can pass Artisan commands:

#### 1. Scaffold a Migration
```bash
cargo run -- make:migration create_posts_table
```
This generates a fresh timestamped `.rs` file in your `migrations/` directory containing an empty `Schema::create` block.

#### 2. Run Migrations
```bash
cargo run -- migrate
```
Rullst connects to the DB, checks the `rullst_migrations` tracker table, and executes all pending `.rs` migrations sequentially.

#### 3. Rollback
```bash
cargo run -- migrate:rollback
```
Reverts the last batch of executed migrations by triggering their `down()` or `drop_if_exists` methods.

#### 4. Audit Table Generation
```bash
cargo run -- make:audit
```
Automatically seeds the `rullst_audits` tracking table into your schema to support the `#[orm(auditable)]` feature.
