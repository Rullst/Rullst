# 3. Advanced Features

Rullst ORM packs an arsenal of enterprise-grade features intended for SaaS scaling, cache management, and strict compliance.

---

## 🏢 Multi-Tenancy

Rullst seamlessly supports single-database Multi-Tenancy out-of-the-box using asynchronous task-local scoping (`tokio::task_local!`).

1. Flag the column in your macro:
```rust
#[derive(Debug, Clone, FromRow, Orm)]
#[orm(tenant_column = "company_id")] // <--- Enables Multi-Tenancy
pub struct User {
    pub id: i32,
    pub company_id: String,
    pub name: String,
}
```

2. Wrap your operations using `with_tenant`. The ORM will automatically apply `WHERE company_id = ?` to all reads, updates, and deletes, and inject the `company_id` onto all new inserts.

```rust
use rullst_orm::tenant::with_tenant;

let results = with_tenant("company_A1", async {
    // This query magically becomes: 
    // SELECT * FROM users WHERE company_id = 'company_A1'
    let users = User::all().await.unwrap(); 

    // This insert magically populates company_id automatically
    let mut new_user = User { id: 0, company_id: "".to_string(), name: "Bob".to_string() };
    new_user.save().await.unwrap();
}).await;
```

---

## 🛡️ Audit Trails

Automatically track a history of row modifications for compliance and debugging. Just add `#[orm(auditable)]`.

```rust
#[derive(Debug, Clone, FromRow, Orm)]
#[orm(auditable)] // <--- Intercepts and logs state diffs
pub struct Post {
    pub id: i32,
    pub title: String,
    pub content: String,
}
```
Whenever a `Post` is saved or deleted, Rullst compares the previous database state with the newly provided struct. If fields changed, it creates a JSON diff (`old_values` / `new_values`) and saves it inside the `rullst_audits` table.

> **Note on PII / Sensitive Fields:** If an `#[orm(auditable)]` model contains known sensitive fields (e.g., `password`, `token`, `secret`, `api_key`), you **must** explicitly flag them with `#[orm(masked)]` or `#[orm(skip)]`. The ORM will enforce this at compile time to ensure no sensitive fields are accidentally logged without acknowledging the masking configuration.

*(Make sure to run the `make:audit` migration to instantiate the table first!)*

---

## 🗑️ Configurable Soft Delete

Inspired by MyBatis-Plus, Rullst now lets you pick **which column** marks a row as deleted, **what value** means "not deleted", and **what value/function** means "deleted". The field type can be anything (`Integer`, `Boolean`, `LocalDateTime`, …) and the generated `SELECT` / `UPDATE` / `restore` SQL stays portable across MySQL, PostgreSQL and SQLite.

### 1. Configure the column

Use the `soft_delete` argument on `#[orm(...)]`:

| Key      | Description                                                                                       | Example                                   |
| -------- | ------------------------------------------------------------------------------------------------- | ----------------------------------------- |
| `field`  | The column name used to mark a soft delete. Defaults to `deleted_at`.                             | `field = "is_deleted"`                    |
| `value`  | The "not deleted" sentinel. Use the literal string `null` to compare against `IS NULL`.           | `value = "0"`, `value = "null"`           |
| `delval` | The "deleted" sentinel. Can be a literal *or* a database function like `now()` / `UNIX_TIMESTAMP()`. | `delval = "1"`, `delval = "now()"`, `delval = "UNIX_TIMESTAMP()"` |

```rust
// Integer flag (0 = active, 1 = deleted)
#[derive(Debug, Clone, Default, FromRow, Orm)]
#[orm(table = "users", soft_delete(field = "is_deleted", value = "0", delval = "1"))]
pub struct User {
    pub id: i32,
    pub name: String,
    pub is_deleted: i32,
}
```

```rust
// `datetime` with `null` for "not deleted" and `now()` for "deleted"
#[derive(Debug, Clone, Default, FromRow, Orm)]
#[orm(table = "posts", soft_delete(field = "deleted_at", value = "null", delval = "now()"))]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub deleted_at: Option<chrono::NaiveDateTime>,
}
```

```rust
// `bigint` row counter — perfect for putting the column inside a UNIQUE index
// (the same row can be "deleted" many times and the value just keeps growing).
#[derive(Debug, Clone, Default, FromRow, Orm)]
#[orm(table = "events", soft_delete(field = "deleted_at", value = "0", delval = "UNIX_TIMESTAMP()"))]
pub struct Event {
    pub id: i32,
    pub payload: String,
    pub deleted_at: i64,
}
```

### 2. Use the API

`User::query()` automatically hides soft-deleted rows. The generated SQL filters by the column/value you configured (e.g. `WHERE is_deleted = 0`):

```rust
let active = User::query().get().await?;                 // hides deleted
let trashed = User::query().only_trashed().get().await?; // only deleted
let all     = User::query().with_trashed().get().await?; // both
```

Soft delete itself flips the column to the `delval` you declared:

```rust
user.delete().await?;     // UPDATE users SET is_deleted = 1 WHERE id = ?
user.restore().await?;    // UPDATE users SET is_deleted = 0 WHERE id = ?
user.force_delete().await?; // DELETE FROM users WHERE id = ?
```

`QueryBuilder::delete_all()` is also smart: it issues an `UPDATE … SET <col> = <delval>` instead of a destructive `DELETE` when the model is soft-delete aware.

### 3. Cross-database notes

- `value = "null"` is rendered as `IS NULL` / `IS NOT NULL` — works on every driver.
- `value = "0"` / `value = "1"` are rendered as `<col> = 0` / `<col> = 1` — portable.
- `delval = "now()"`, `delval = "CURRENT_TIMESTAMP"`, `delval = "UNIX_TIMESTAMP()"` are interpolated **verbatim** as raw SQL. Pick the function your database actually supports.
- The pre-existing `deleted_at` model (no `#[orm(soft_delete(...))]`) still compiles: it is treated as `value = ""` (compared as `IS NULL`) and `delval = "CURRENT_TIMESTAMP"`, matching the historical behaviour.

---

## 🙈 Skipping Fields From Generated SQL

Add `#[sqlx(skip)]` (or the alias `#[orm(skip)]`) to a struct field to tell the macro **the column does not exist in the table**. The field stays on the struct so you can still read/write it locally, but the macro excludes it from:

- `INSERT` / `UPDATE` column lists and bindings
- the `*Column` enum
- JSON (`to_json` / `from_json_value`) and the cache serialisers
- the `sqlx::FromRow` mapping (so missing-column errors at runtime disappear)

This is the same pattern used in `sqlx` itself and is the recommended way to attach derived/local-only state (e.g. computed flags, secrets, in-memory caches) to a model.

```rust
#[derive(Debug, Clone, Default, FromRow, Orm)]
#[orm(table = "users", soft_delete(field = "is_deleted", value = "0", delval = "1"))]
pub struct User {
    pub id: i32,
    pub name: String,
    pub is_deleted: i32,

    // `secret` is intentionally not persisted. The macro removes it
    // from INSERT / UPDATE column lists, the `*Column` enum, and
    // FromRow, while still letting you read/write `user.secret`
    // locally.
    #[sqlx(skip)]
    pub secret: String,
}
```

> **Note:** when a model declares any `#[sqlx(skip)]` (or `#[orm(skip)]`) field, the generated `from_json_value` falls back to `..Default::default()` for the trailing fields, so the model must also `derive(Default)`.

### Compile-time vs. runtime exclusion

A `#[sqlx(skip)]` field is excluded at *two* levels:

1. **Compile time.** The `*Column` enum does not have a variant for
   the skipped field, and the `where_<field>`, `or_where_<field>`,
   `where_not_<field>`, `order_by_<field>`, `order_by_<field>_desc`
   magic methods are never generated. There is no way to spell
   `UserColumn::Secret` or `User::query().where_secret(...)` — Rust
   refuses to compile.
2. **Runtime.** The raw string-based builders
   (`where_eq` / `where_not_eq` / `where_gt` / `where_lt` /
   `where_like` / `where_not_like` / `where_null` / `where_not_null` /
   `where_in` / `where_not_in` / `where_between` / `where_not_between`
   / `or_where_*` / `group_by` / `order_by` / `order_by_desc` /
   `select`) would happily emit `WHERE secret = ?` if you handed
   them the column name as a string. The generated builder now
   captures the list of skipped column names in a `const
   SKIPPED_COLUMNS: &'static [&'static str]` and rejects any
   reference to them with a `Validation` error before the SQL is
   built:

   ```text
   Validation error: column `secret` is declared with
   `#[orm(skip)]` / `#[sqlx(skip)]` and does not exist in the table;
   it must not be used in WHERE / ORDER BY / GROUP BY / SELECT
   ```

   This means the typed API, the magic methods, the column enum, and
   the raw string API all agree: a `#[sqlx(skip)]` field is invisible
   to the database.

A runnable example lives in `rullst-orm/examples/custom_soft_delete.rs`:

```bash
cargo run -p rullst-orm --example custom_soft_delete
```

---

## 🔎 Scout Full-Text Search

If you enable Scout on a model, Rullst will automatically index it into Meilisearch/Algolia via API, or default to a powerful internal `LIKE` engine if no external backend is attached.

```rust
#[derive(Debug, Clone, FromRow, Orm)]
#[orm(scout)]
pub struct Article {
    pub id: i32,
    pub body: String,
}

// Perform a full-text search safely!
let hits = Article::search("Rust language").get().await?;
```

---

## 🚀 Read/Write Splitting & Caching

If you operate on distributed databases or require extreme speeds, Rullst splits traffic natively.

### Replicas
Initialize read replicas alongside your primary database. Rullst automatically sends `SELECT` queries in a round-robin format to replicas, keeping `INSERT/UPDATE/DELETE` on the primary instance.
```rust
Orm::init("postgres://master").await?;
Orm::init_replicas(&["postgres://replica_1", "postgres://replica_2"]).await?;
```

### Redis `remember()`
If you compiled the crate with the `redis` feature, you can cache any expensive SQL query instantly:

```rust
Orm::init_redis("redis://127.0.0.1/").await?;

// First query hits Postgres. All queries for the next 3600 seconds hit Redis!
let stats = Analytics::query()
    .remember(3600)
    .get()
    .await?;
```
