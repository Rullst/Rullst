# Pull Request

## Title

`feat(macros): configurable soft delete + #[sqlx(skip)] field attribute`

## Description

Two new ergonomic knobs on `#[derive(rullst_orm::Orm)]`, both inspired by
MyBatis-Plus:

1. **Configurable soft delete** — pick the column, the "not deleted"
   sentinel, and the "deleted" sentinel (a literal *or* a database
   function like `now()` / `UNIX_TIMESTAMP()`).
2. **`#[sqlx(skip)]` (alias of `#[orm(skip)]`)** — exclude a struct
   field from generated SQL while keeping the field on the struct for
   local use.

The generated `SELECT` / `UPDATE` / `restore` SQL is portable across
MySQL, PostgreSQL and SQLite (no dialect-specific branches).

---

## Motivation

Previously, soft delete was hard-wired to a `deleted_at` column compared
with `IS NULL`, and there was no way to opt a column out of generated
SQL without losing the field on the struct. These two limitations
forced users to drop down to raw `sqlx` for a class of models that is
very common in production (boolean / integer flag columns, bigint
monotonic counters inside unique indexes, in-memory caches on a model,
etc.).

This PR brings the MyBatis-Plus ergonomics that are described in the
issue to Rust, on top of `rullst-orm`'s existing builder.

---

## Changes

### 1. New `#[orm(soft_delete(field, value, delval))]` configuration

| Key      | Description                                                                                          | Example                                       |
| -------- | ---------------------------------------------------------------------------------------------------- | --------------------------------------------- |
| `field`  | Column name used as the soft delete marker. Defaults to `deleted_at`.                                | `field = "is_deleted"`                        |
| `value`  | "Not deleted" sentinel. The literal string `null` renders as `IS NULL` / `IS NOT NULL`.               | `value = "0"`, `value = "null"`               |
| `delval` | "Deleted" sentinel. Spliced verbatim as raw SQL so users can pass any database function.              | `delval = "1"`, `delval = "now()"`, `delval = "UNIX_TIMESTAMP()"` |

```rust
// Integer flag (0 = active, 1 = deleted)
#[derive(Debug, Clone, Default, FromRow, Orm)]
#[orm(table = "users", soft_delete(field = "is_deleted", value = "0", delval = "1"))]
pub struct User {
    pub id: i32,
    pub name: String,
    pub is_deleted: i32,
}

// `datetime` with `null` for "not deleted" and `now()` for "deleted"
#[derive(Debug, Clone, Default, FromRow, Orm)]
#[orm(table = "posts", soft_delete(field = "deleted_at", value = "null", delval = "now()"))]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub deleted_at: Option<chrono::NaiveDateTime>,
}

// `bigint` counter (unique-index friendly, supports multi-delete)
#[derive(Debug, Clone, Default, FromRow, Orm)]
#[orm(table = "events", soft_delete(field = "deleted_at", value = "0", delval = "UNIX_TIMESTAMP()"))]
pub struct Event {
    pub id: i32,
    pub payload: String,
    pub deleted_at: i64,
}
```

API:

```rust
let active  = User::query().get().await?;                  // hides deleted
let trashed = User::query().only_trashed().get().await?;  // only deleted
let all     = User::query().with_trashed().get().await?;  // both

user.delete().await?;       // UPDATE users SET is_deleted = 1 WHERE id = ?
user.restore().await?;      // UPDATE users SET is_deleted = 0 WHERE id = ?
user.force_delete().await?; // DELETE FROM users WHERE id = ?
```

`QueryBuilder::delete_all()` is also smart: it emits
`UPDATE <table> SET <col> = <delval> …` instead of a destructive
`DELETE` when the model is soft-delete aware.

**Cross-database behaviour:**

- `value = "null"` ⇒ `IS NULL` / `IS NOT NULL` (works on every driver).
- `value = "0"` / `value = "1"` ⇒ `<col> = 0` / `<col> = 1` (portable).
- `delval = "now()"`, `delval = "CURRENT_TIMESTAMP"`,
  `delval = "UNIX_TIMESTAMP()"` are interpolated verbatim; pick the
  function your database actually supports.
- Pre-existing `deleted_at` models (no `#[orm(soft_delete(...))]`) keep
  compiling and behaving the same way (`IS NULL` /
  `CURRENT_TIMESTAMP`).

### 2. New `#[sqlx(skip)]` field attribute

```rust
#[derive(Debug, Clone, Default, FromRow, Orm)]
#[orm(table = "users", soft_delete(field = "is_deleted", value = "0", delval = "1"))]
pub struct User {
    pub id: i32,
    pub name: String,
    pub is_deleted: i32,

    /// `secret` is intentionally not persisted. The macro removes it
    /// from INSERT / UPDATE column lists, the `*Column` enum, the JSON
    /// serialiser, and the row mapping, while still letting you
    /// read/write `user.secret` locally.
    #[sqlx(skip)]
    pub secret: String,
}
```

The skipped field is excluded from:

- `INSERT` / `UPDATE` column lists and bindings
- the generated `*Column` enum
- `to_json` / `from_json_value` and the cache serialisers
- the `sqlx::FromRow` mapping (so missing-column errors disappear)

`#[orm(skip)]` remains supported as an alias so the existing surface
keeps working. Models with any skipped field get a
`..Default::default()` tail in `from_json_value`, so they must also
`derive(Default)`.

---

## Files changed

| File                                                       | What changed                                                         |
| ---------------------------------------------------------- | -------------------------------------------------------------------- |
| `rullst-orm-macros/src/parser.rs`                          | Parse `soft_delete(field, value, delval)` and `#[orm(skip)]`/`#[sqlx(skip)]` |
| `rullst-orm-macros/src/builder.rs`                         | Render portable `WHERE` filters + `delete_all` `UPDATE`; pre-render `IS NULL` / `= <value>` fragments |
| `rullst-orm-macros/src/models.rs`                          | Generate `delete` / `restore` / `force_delete` honouring the config; emit `..Default::default()` when a skip field is present |
| `rullst-orm-macros/src/lib.rs`                             | Accept `sqlx` as a top-level attribute alongside `orm`              |
| `rullst-orm-macros/tests/macro_tests.rs`                   | 6 new tests: explicit config, null sentinel, bigint timestamp, both skip aliases, combined usage |
| `rullst-orm/tests/integration_tests.rs`                    | `scenario_configurable_soft_delete` + `scenario_skipped_field` end-to-end |
| `rullst-orm/examples/custom_soft_delete.rs`                | New runnable demo covering insert / read / soft delete / restore / force_delete |
| `docs/3-advanced-features.md`                              | New `🗑️ Configurable Soft Delete` and `🙈 Skipping Fields From Generated SQL` sections |
| `.gitignore`                                               | Ignore `.idea/`                                                      |

---

## How to verify

```bash
# 1. Unit + macro tests
cargo test -p rullst-orm-macros

# 2. End-to-end integration tests (SQLite)
cargo test -p rullst-orm --test integration_tests

# 3. The runnable example
cargo run -p rullst-orm --example custom_soft_delete
```

All tests pass on the local machine:

```
running 10 tests
test test_model_with_explicit_soft_delete_config ... ok
test test_model_with_soft_delete_bigint_timestamp ... ok
test test_model_with_orm_skip_field ... ok
test test_model_with_relations ... ok
test test_model_with_combined_soft_delete_and_skip ... ok
test test_basic_model ... ok
test test_model_with_sqlx_skip_field ... ok
test test_model_with_hidden_fields ... ok
test test_model_with_soft_delete_null_sentinel ... ok
test test_model_with_soft_deletes ... ok

running 1 test
test integration_suite ... ok
```

---

## Type of change

- [ ] Bug fix (non-breaking change which fixes an issue)
- [x] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing
      functionality to not work as expected)
- [x] This change requires a documentation update

> The new `soft_delete(field, value, delval)` is opt-in (existing
> `deleted_at` models continue to behave identically). The new
> `#[sqlx(skip)]` is opt-in (existing models are unchanged).
> The macro now also accepts the `sqlx` attribute path, but `orm`
> continues to work exactly as before.

## Checklist

- [x] My code follows the style guidelines of this project (`cargo fmt`,
      `cargo clippy`)
- [x] I have performed a self-review of my own code
- [x] I have commented my code, particularly in hard-to-understand areas
- [x] I have made corresponding changes to the documentation
- [x] My changes generate no new warnings (`cargo check` and
      `cargo check --tests` are clean on the workspace)
- [x] I have added tests that prove my fix is effective or that my
      feature works
- [x] New and existing unit tests pass locally with my changes
