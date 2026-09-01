# Tutorial 03: Active Record CRUD Operations 🗄️

`rullst-orm` derives a typed query builder and persistence methods from a Rust
struct. The SQLx-backed model uses an `i32` primary key named `id`; a zero value
means that `save()` inserts, while a non-zero value means that it updates.

---

## Step 1: Define an Active Record model

In `src/models/user.rs`:

```rust
use rullst_orm::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "users")]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
}
```

The database table must contain matching columns. Run migrations before using
the model.

---

## Step 2: Perform CRUD operations

### Create

```rust,no_run
# use rullst_orm::{FromRow, Orm};
# #[derive(Debug, Clone, FromRow, Orm)]
# #[orm(table = "users")]
# struct User { id: i32, name: String, email: String }
# async fn create_user() -> Result<(), rullst_orm::Error> {
let mut user = User {
    id: 0,
    name: "Alice Developer".to_string(),
    email: "alice@example.com".to_string(),
};
user.save().await?;
// `user.id` now contains the inserted primary key.
# Ok(())
# }
```

### Read and filter

```rust,no_run
# use rullst_orm::{FromRow, Orm};
# #[derive(Debug, Clone, FromRow, Orm)]
# #[orm(table = "users")]
# struct User { id: i32, name: String, email: String }
# async fn read_users() -> Result<(), rullst_orm::Error> {
let user = User::find(1).await?; // Result<Option<User>, rullst_orm::Error>
let example_users = User::query()
    .where_like("email", "%@example.com")
    .get()
    .await?;
# let _ = (user, example_users);
# Ok(())
# }
```

Builder values are bound as query parameters. Column names are validated as
identifiers, but they should still be application-owned constants rather than
untrusted request input.

### Update

```rust,no_run
# use rullst_orm::{FromRow, Orm};
# #[derive(Debug, Clone, FromRow, Orm)]
# #[orm(table = "users")]
# struct User { id: i32, name: String, email: String }
# async fn update_user() -> Result<(), rullst_orm::Error> {
if let Some(mut user) = User::find(1).await? {
    user.name = "Alice Smith".to_string();
    user.save().await?;
}
# Ok(())
# }
```

### Delete

```rust,no_run
# use rullst_orm::{FromRow, Orm};
# #[derive(Debug, Clone, FromRow, Orm)]
# #[orm(table = "users")]
# struct User { id: i32, name: String, email: String }
# async fn delete_user() -> Result<(), rullst_orm::Error> {
if let Some(user) = User::find(1).await? {
    user.delete().await?;
}
# Ok(())
# }
```

---

## Soft deletes

A `deleted_at: Option<String>` field opts the model into the default soft-delete
contract. For a different sentinel, configure it explicitly and ensure the
migration uses the same representation:

```rust
use rullst_orm::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(
    table = "users",
    soft_delete(field = "is_deleted", value = "0", delval = "1")
)]
pub struct SoftUser {
    pub id: i32,
    pub name: String,
    pub is_deleted: i32,
}
```

`query()` applies the model's configured scopes. `unscoped()` is an explicit
administrative escape hatch and should not be used directly from request data.

---

## Key takeaways

- `save`, `find`, `all`, `query`, and `delete` are generated for SQLx-backed
  `Orm` models.
- Creation uses a normal Rust struct, not a JSON map.
- CRUD errors are returned as `rullst_orm::Error`; a missing row is `Ok(None)`.
- Use a caller-owned transaction and the generated `*_with_tx` methods when a
  business operation must commit multiple writes atomically.
