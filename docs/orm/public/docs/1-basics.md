# 1. Basics & Query Builder

Rullst ORM is built around a powerful, auto-generated Query Builder. When you add `#[derive(Orm)]` to your struct, it injects an entire fluent API for querying the database.

## Model Definition

Your structs must derive `Orm`, `FromRow` (from `sqlx`), and standard Rust traits like `Debug` and `Clone`.

```rust
use rullst_orm::{Orm, FromRow};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "users")] // Optional: specifies table name. Defaults to lowercase plural (e.g. users)
pub struct User {
    pub id: i32, // Primary key
    pub email: String,
    pub is_active: bool,
}
```

## Connecting to the Database

Initialize the global connection pool exactly once at application startup. Rullst ORM dynamically supports `postgres`, `mysql`, and `sqlite` based on your connection string format.

```rust
use rullst_orm::Orm;

#[tokio::main]
async fn main() -> Result<(), rullst_orm::Error> {
    Orm::init("postgres://user:pass@localhost/db").await?;
    // OR: Orm::init("mysql://user:pass@localhost/db").await?;
    // OR: Orm::init("sqlite://app.db").await?;
    // OR: Orm::init("sqlite::memory:").await?;
    
    Ok(())
}
```

## Active Record CRUD

These operations can be called directly on the model instance:

```rust
// 1. Find by ID
let user = User::find(1).await?;
let user = User::find_or_fail(1).await?;

// 2. Insert (If id == 0, it runs INSERT)
let mut user = User { id: 0, email: "john@doe.com".to_string(), is_active: true };
user.save().await?; 

// 3. Update (If id > 0, it runs UPDATE)
user.is_active = false;
user.save().await?;

// 4. Delete
user.delete().await?;
```

## Query Builder

To start a fluent chain, call `.query()`.

### Filtering (WHERE)
All `.where_*` methods automatically bind the parameter securely to prevent SQL Injection.

```rust
let active_users = User::query()
    .where_eq("is_active", true)
    .where_not_null("email")
    .where_like("email", "%@company.com")
    .get()
    .await?;
```

**Available Filters:**
- `.where_eq(col, val)`, `.where_not_eq(col, val)`
- `.where_gt(col, val)`, `.where_lt(col, val)`
- `.where_like(col, val)`
- `.where_in(col, vec!)`, `.where_between(col, min, max)`
- Prefix any with `or_` (e.g., `.or_where_eq()`)

### Raw Queries (Security Warning)
If you must use `.where_raw()`, **never concatenate strings**. You must pass an array of bindings directly to the method:

```rust
// ✅ SECURE
let users = User::query()
    .where_raw("email = ? AND is_active = ?", vec!["admin@example.com".into(), true.into()])
    .get()
    .await?;
```

### Execution Methods
Terminate your query builder chain to hit the database:
- `.get().await?` -> Returns `Vec<Model>` *(Note: Limited to 1000 rows and 30s timeout by default for security. Use `.unsafe_unlimited()` to bypass the row limit).*
- `.first().await?` -> Returns `Option<Model>` (adds `LIMIT 1`)
- `.count().await?` -> Returns `i64`
- `.paginate(page, per_page).await?` -> Returns a struct containing metadata and the slice of data.
- `.delete_all().await?` -> Deletes all matching rows.

### Memory Safe Processing (Chunk)
If you need to process millions of rows, use `.chunk()` to yield small blocks into memory at a time.

```rust
User::query()
    .where_eq("is_active", true)
    .chunk(1000, |batch| Box::pin(async move {
        for user in batch {
            println!("User: {}", user.email);
        }
    }))
    .await?;
```
