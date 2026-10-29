# Tutorial 03: Active Record CRUD Operations 🗄️

`rullst-orm` derives a typed query builder and persistence methods from a Rust
struct. The SQLx-backed model uses an `i32` primary key named `id`; a zero value
means that `save()` inserts, while a non-zero value means that it updates.

This tutorial continues in the `shop` project of
[Tutorial 02](02-cli-generators.md), whose `cargo rullst make:model Product
--migration` wrote `src/models/product.rs` and
`src/migrations/m<timestamp>_create_products.rs`.

---

## Step 1: Define an Active Record model

Give the generated migration the columns the model will use. In its `up()`:

```rust,ignore
Schema::create("products", |table| {
    table.id();
    table.string("name").not_null();
    table.integer("price_cents").not_null();
    table.timestamps();
}).await
```

Then add the matching fields to `src/models/product.rs`:

```rust
use rullst::db::{FromRow, Orm, RowSnapshot};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "products")]
pub struct Product {
    pub id: i32,
    pub name: String,
    pub price_cents: i32,
    // Remembers the loaded values, so `save()` writes only the columns this
    // record changed. Build new records with `snapshot: RowSnapshot::default()`.
    #[sqlx(skip)]
    pub snapshot: RowSnapshot,
}
```

Apply the migration with `cargo rullst db:migrate` (or restart
`cargo rullst dev`) before using the model. The model leaves out
`created_at`/`updated_at`, so inserts keep their database defaults. If
`cargo rullst dev` already applied the migration while it had only an `id`,
run `cargo rullst db:rollback` (it reverts the last batch of migrations,
including this one) and then `cargo rullst db:migrate`;
[Tutorial 05](05-migrations-and-seeds.md) explains both commands.

---

## Step 2: Perform CRUD operations

Call these methods from an `async` handler or seeder, for example in a
controller of [Tutorial 02](02-cli-generators.md).

### Create

```rust,no_run
# use rullst::db::{FromRow, Orm, RowSnapshot};
# #[derive(Debug, Clone, FromRow, Orm)]
# #[orm(table = "products")]
# struct Product { id: i32, name: String, price_cents: i32, #[sqlx(skip)] snapshot: RowSnapshot }
# async fn create_product() -> Result<(), rullst::orm::Error> {
let mut product = Product {
    id: 0,
    name: "Starter kit".to_string(),
    price_cents: 2_500,
    snapshot: RowSnapshot::default(),
};
product.save().await?;
// `product.id` now contains the inserted primary key.
# Ok(())
# }
```

### Read and filter

```rust,no_run
# use rullst::db::{FromRow, Orm, RowSnapshot};
# #[derive(Debug, Clone, FromRow, Orm)]
# #[orm(table = "products")]
# struct Product { id: i32, name: String, price_cents: i32, #[sqlx(skip)] snapshot: RowSnapshot }
# async fn read_products() -> Result<(), rullst::orm::Error> {
let product = Product::find(1).await?; // Result<Option<Product>, rullst::orm::Error>
let kits = Product::query()
    .where_like("name", "%kit%")
    .get()
    .await?;
# let _ = (product, kits);
# Ok(())
# }
```

Builder values are bound as query parameters. Column names are validated as
identifiers, but they should still be application-owned constants rather than
untrusted request input.

### Update

```rust,no_run
# use rullst::db::{FromRow, Orm, RowSnapshot};
# #[derive(Debug, Clone, FromRow, Orm)]
# #[orm(table = "products")]
# struct Product { id: i32, name: String, price_cents: i32, #[sqlx(skip)] snapshot: RowSnapshot }
# async fn update_product() -> Result<(), rullst::orm::Error> {
if let Some(mut product) = Product::find(1).await? {
    product.price_cents = 2_000;
    // The snapshot makes this an UPDATE of `price_cents` only.
    product.save().await?;
}
# Ok(())
# }
```

### Delete

```rust,no_run
# use rullst::db::{FromRow, Orm, RowSnapshot};
# #[derive(Debug, Clone, FromRow, Orm)]
# #[orm(table = "products")]
# struct Product { id: i32, name: String, price_cents: i32, #[sqlx(skip)] snapshot: RowSnapshot }
# async fn delete_product() -> Result<(), rullst::orm::Error> {
if let Some(product) = Product::find(1).await? {
    product.delete().await?;
}
# Ok(())
# }
```

Without a `RowSnapshot` field, `save()` on a loaded record writes every
column, so the last writer wins; see the
[ORM guide](../crates/orm.md#concurrent-updates-and-counters) for dirty
tracking and `update_partial()`.

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

`restore()` clears the soft-delete marker and `force_delete()` removes the row
permanently. Both check the tenant and the policy (`can_restore` /
`can_force_delete`) first and then run in a savepoint of the task-scoped
`Orm::transaction`, or in their own transaction, like `delete()`:

- `force_delete()` runs the `before_delete`/`after_delete` hooks and the
  `deleting`/`deleted` observers, writes a `force_deleted` audit entry on
  auditable models, and after commit invalidates the query cache, publishes the
  Redis `deleted` event, calls `committed(Deleted)` and removes the Scout
  document. It does not cascade to `cascade_soft_delete` relations.
- `restore()` re-reads the restored row, calls the `updated` and `saved`
  observers with it, writes a `restored` audit entry, and after commit
  invalidates the cache, publishes the Redis `updated`/`saved` events, calls
  `committed(Updated)` and re-indexes the Scout document. The save hooks and
  the `saving`/`updating` observers are not called, because restore writes
  only the soft-delete column. Restoring a missing or live row is a no-op,
  and deleting an already trashed row again fails without changing its
  deletion time.

---

## Key takeaways

- `save`, `find`, `all`, `query`, and `delete` are generated for SQLx-backed
  `Orm` models.
- Creation uses a normal Rust struct, not a JSON map.
- CRUD errors are returned as `rullst::orm::Error` (the `rullst_orm::Error`
  type); a missing row is `Ok(None)`.
  Saving or deleting a handle whose row was deleted meanwhile returns
  `RecordNotFound` instead of silently succeeding.
- Use a caller-owned transaction and the generated `*_with_tx` methods when a
  business operation must commit multiple writes atomically.
