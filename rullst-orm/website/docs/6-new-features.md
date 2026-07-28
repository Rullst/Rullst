# 6. New Features Showcase (v6+)

Rullst ORM has reached a new level of productivity with advanced developer experience enhancements.

---

## 🏗️ Database-First Introspection (`cargo rullst generate:models`)

Migrating a legacy database to a new ORM is usually painful. With Rullst, you can automatically introspect your existing PostgreSQL, MySQL, or SQLite databases and generate production-ready `#[derive(Orm)]` Rust structs.

```bash
# Generate models from an existing database
cargo rullst generate:models --driver sqlite --url sqlite::memory: --output ./src/models
```
The CLI automatically maps SQL types (`VARCHAR`, `INT`, `BOOLEAN`) into their native Rust counterparts (`String`, `i32`, `bool`).

---

## 🚫 Strict Lazy Loading Prevention

The N+1 query problem is the silent killer of application performance. Rullst ORM can now physically prevent this in your development and testing environments by panicking the application if a relation is accessed without being eager-loaded first.

1. Enable the prevention logic globally (e.g., in your `main.rs`):
```rust
use rullst_orm::prevent_lazy_loading;

#[tokio::main]
async fn main() {
    // Only enable this in development/testing!
    if cfg!(debug_assertions) {
        prevent_lazy_loading(true);
    }
}
```

2. What happens next?
```rust
// ❌ This will PANIC because the "posts" relation was not eager-loaded via .with("posts")
let user = User::query().first().await.unwrap();
let posts = user.posts().await; // PANIC: "Lazy loading is prevented..."

// ✅ This works perfectly
let user = User::query().with("posts").first().await.unwrap();
let posts = user.posts().await; // Returns the pre-loaded posts instantly.
```

---

## 🔒 Model Policies (Authorization)

You can tightly couple access control logic to your models using Laravel-style Policies.

1. Define a `Policy`:
```rust
use rullst_orm::policy::Policy;
use async_trait::async_trait;

pub struct PostPolicy;

#[async_trait]
impl Policy<Post> for PostPolicy {
    async fn can_update(model: &Post) -> bool {
        // Your logic here (e.g., check if the user is the owner)
        true 
    }
    async fn can_delete(model: &Post) -> bool {
        false // Deny deletion
    }
}
```

2. Bind the Policy to your model using `#[orm(policy = "PostPolicy")]`:
```rust
#[derive(Debug, Clone, FromRow, Orm)]
#[orm(policy = "PostPolicy")]
pub struct Post {
    pub id: i32,
    pub title: String,
}
```

3. The ORM intercepts operations. If you attempt to call `post.save().await` or `post.delete().await`, the ORM executes your policy. If access is denied, it returns an authorization error before executing any SQL.

---

## ✨ Type-Safe Partial Updates (Dirty Checking)

Sometimes you want to update just one or two columns without sending the entire struct to the database (which avoids overriding concurrent changes from other users). 

Rullst ORM automatically generates an `UpdateBuilder` for every model.

```rust
// Fetch a user
let mut user = User::query().first().await.unwrap();

// Update only specific fields type-safely!
user.update_partial()
    .name("New Name".to_string())
    .save() // Executes: UPDATE users SET name = $1 WHERE id = $2
    .await?;
```
The macro dynamically tracks which fields you chain into the builder, guaranteeing that `None` values are simply ignored from the `UPDATE` query.

---

## 🔠 Native Enum Mapping

Rullst ORM supports seamless and type-safe integration of Rust `enum` types with your database tables.

1. Define your Enum and derive `Enum`:
```rust
use rullst_orm::Enum;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Enum, Serialize, Deserialize)]
pub enum Status {
    Active,
    Inactive,
    Archived,
}
```

2. Use it directly in your Models:
```rust
#[derive(Debug, Clone, FromRow, Orm)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub status: Status, // Safely mapped to string in the database
}
```

3. In your migrations, use `enum_col` to define the column. This automatically generates cross-database compatible constraints (like `TEXT CHECK` for SQLite or native `ENUM` logic for PostgreSQL/MySQL):
```rust
Schema::create("users", |t| {
    t.id();
    t.string("name");
    // Creates a column restricted to these values
    t.enum_col("status", vec!["Active", "Inactive", "Archived"]);
}).await?;
```
