<div align="center">
  <h1>Rullst ORM 🌟</h1>
  <p><strong>A beautiful, type-safe, Active Record ORM for Rust.</strong></p>

  <p>
    <a href="https://crates.io/crates/rullst-orm"><img src="https://img.shields.io/crates/v/rullst-orm?style=flat-square&color=orange" alt="Crates.io" /></a>
    <a href="https://crates.io/crates/rullst-orm"><img src="https://img.shields.io/crates/d/rullst-orm?style=flat-square&color=orange" alt="Downloads" /></a>
    <a href="https://docs.rs/rullst-orm"><img src="https://img.shields.io/docsrs/rullst-orm?style=flat-square&color=blue" alt="Docs.rs" /></a>
    <a href="https://github.com/Rullst/Rullst/actions"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/ci.yml?style=flat-square&label=Build" alt="Build Status" /></a>
    <img src="https://img.shields.io/badge/License-MIT-yellow.svg?style=flat-square" alt="License: MIT" />
  </p>
</div>

> [!IMPORTANT]
> This page describes the v13 prerelease source, and its dependency examples
> pin the published `13.0.0-alpha.4` exactly: Cargo never selects a prerelease
> on its own, and a plain `"13.0.0-alpha.4"` would also accept later alphas
> (see the [v13 migration guide](../migration-v13.md)). The v12 line
> (`rullst-orm@12`) reached end of life on 16 October 2026; see the
> [release record](../v12.md). Use a path dependency only for checkout-local
> review.

🚀 **[Visit the Official Website & Documentation Hub](https://rullst.github.io/Rullst/book/)** 🚀

Built on top of `sqlx` and procedural macros, **Rullst ORM** brings the delightful, fluent syntax of Active Record frameworks directly to the high-performance Rust ecosystem.

<div align="center">
  <h3>🛡️ Security Engineering</h3>
  <p>Rullst ORM uses SQLx bindings, validated identifiers, typed errors, and layered CI checks. Workflow badges are scoped test results, not a guarantee for an application or deployment.</p>

| Security Audit | Status | Description |
| :--- | :---: | :--- |
| **OpenSSF Scorecard** | <a href="https://scorecard.dev/viewer/?uri=github.com/Rullst/Rullst"><img src="https://img.shields.io/badge/dynamic/json?url=https%3A%2F%2Fapi.scorecard.dev%2Fprojects%2Fgithub.com%2FRullst%2FRullst&query=%24.score&label=OpenSSF%20Scorecard&style=flat-square" alt="OpenSSF Scorecard" /></a> | Current public supply-chain practice score; not a security certification |
| **Release Provenance** | <a href="https://github.com/Rullst/Rullst/actions/workflows/release.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/release.yml?style=flat-square&label=" alt="Release provenance" /></a> | Provenance attestations for release artifacts; no SLSA level is claimed here |
| **Codecov** | <a href="https://codecov.io/gh/Rullst/Rullst"><img src="https://codecov.io/github/Rullst/Rullst/branch/main/graph/badge.svg?component=framework_libraries" alt="Framework library coverage" /></a> | Blocking 90% target for the measured framework-library scope; the complete repository aggregate now also has its own 90% gate |
| **Matrix DB Tests** | <a href="https://github.com/Rullst/Rullst/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/ci.yml?style=flat-square&label=" alt="Testcontainers" /></a> | Live PostgreSQL, MySQL, MariaDB, MongoDB, SurrealDB and libSQL contracts, plus in-process DuckDB tests |
| **OpenSSF** | <a href="https://www.bestpractices.dev/projects/13359"><img src="https://img.shields.io/cii/level/13359?style=flat-square&label=" alt="OpenSSF Best Practices" /></a> | Open source security standards |
| **Property tests** | <a href="https://github.com/Rullst/Rullst/actions/workflows/proptest.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/proptest.yml?branch=main&style=flat-square&label=Proptest" alt="Proptest" /></a> | Scheduled/manual bounded invariant evidence |
| **Miri research matrix** | <a href="https://github.com/Rullst/Rullst/actions/workflows/miri.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/miri.yml?branch=main&style=flat-square&label=Miri" alt="Miri" /></a> | Manual bounded evidence; the selected pure-Rust privacy scope is strict, while native database FFI remains outside Miri |
| **Kani research harnesses** | <a href="https://github.com/Rullst/Rullst/actions/workflows/kani.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/kani.yml?branch=main&style=flat-square&label=Kani" alt="Kani" /></a> | Manual, bounded formal evidence; not whole-ORM proof |
| **CodeQL SAST** | <a href="https://github.com/Rullst/Rullst/actions/workflows/codeql.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/codeql.yml?style=flat-square&label=" alt="CodeQL SAST" /></a> | Advanced semantic code analysis |
| **Cargo Deny** | <a href="https://github.com/Rullst/Rullst/actions/workflows/cargo-deny.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/cargo-deny.yml?style=flat-square&label=" alt="Cargo Deny" /></a> | Banning unmaintained/vulnerable crates |
| **Cargo Audit** | <a href="https://github.com/Rullst/Rullst/actions/workflows/audit.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/audit.yml?style=flat-square&label=" alt="Auto-Audit" /></a> | Continuous scanning for crate vulnerabilities |
| **Cargo SemVer** | <a href="https://github.com/Rullst/Rullst/actions/workflows/semver.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/semver.yml?style=flat-square&label=" alt="cargo-semver-checks" /></a> | Strict SemVer API breakage checks |
| **Cargo Machete** | <a href="https://github.com/Rullst/Rullst/actions/workflows/machete.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/machete.yml?style=flat-square&label=" alt="Cargo Machete" /></a> | Detecting unused and bloated dependencies |
| **On-demand fuzzing** | <a href="https://github.com/Rullst/Rullst/actions/workflows/fuzzing.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/fuzzing.yml?branch=main&style=flat-square&label=Fuzzing" alt="Fuzzing" /></a> | Manual time-bounded targets; no continuous OSS-Fuzz claim |
| **Mutation Testing** | <a href="https://github.com/Rullst/Rullst/actions/workflows/mutants.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/mutants.yml?style=flat-square&label=" alt="Mutants" /></a> | Mutation testing for test suite robustness |
| **Continuous Benchmarks** | <a href="https://github.com/Rullst/Rullst/actions/workflows/bench.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/bench.yml?style=flat-square&label=" alt="Benchmarks CI" /></a> | Continuous performance regression testing & live dashboard |
| **Unsafe Policy** | <a href="https://github.com/Rullst/Rullst/actions/workflows/unsafe-policy.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/unsafe-policy.yml?style=flat-square&label=" alt="Unsafe Policy" /></a> | Audits unsafe usage within the workflow's declared scope |
| **Panic Policy** | <a href="https://github.com/Rullst/Rullst/actions/workflows/zero-panics.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/zero-panics.yml?style=flat-square&label=" alt="Panic Policy" /></a> | Graceful error handling across the framework |



</div>

## 🚀 Why Rullst ORM?

Rullst ORM generates Active Record operations and a fluent query builder from
`#[derive(Orm)]`. SQLx remains available for queries that do not fit the
generated API. Both work with SQLite, PostgreSQL and MySQL/MariaDB; Turso/libSQL
and the side stores have their own APIs, as
[Database Support](../database-support.md) lists.

**Key Features:**
- **Generated CRUD:** Insert, update, delete, restore, and find operations for
  supported model shapes. `restore()` and `force_delete()` use the same
  savepoint, hook/observer, audit and post-commit cache/event/Scout pipeline as
  `delete()`/`save()`; see [Active Record CRUD](../tutorials/03-active-record-crud.md).
- **Fluent Query Builder:** Chain methods such as `.where_eq()`, `.limit()`, and
  `.order_by()`; values are bound and structural identifiers are validated.
- **Relationships and eager loading:** `has_many`, `has_one`, `belongs_to`, and
  polymorphic relationship helpers, with explicit eager-load methods.
- **Opt-in tenant scope:** `#[orm(tenant_column = "account_id")]` adds the
  configured task-local tenant to generated model queries. Applications must
  establish the tenant context at their authenticated boundary.
- **Actor-bound audit revisions:** `#[orm(auditable)]` requires a validated
  user/service/system `AuditContext`; the active tenant and optional correlation
  ID are recorded with recursively redacted bounded changes. Generated
  instance saves/deletes/restores/force-deletes and their audit entry share a
  savepoint and fail together. Eligible v2 updates expose guarded revision restoration, which
  rejects stale, cross-tenant, redacted, malformed, legacy, create/delete, and
  oversized revisions and records a compensating audit entry. The host still
  derives authenticated principal/tenant authority, while bulk per-row history
  and durable export remain explicit. See
  [Auditable Revisions](../tutorials/50-auditable-revisions.md).
- **Field privacy:** `#[orm(encrypted)]` transparently encrypts supported
  `String` fields with a versioned AES-256-GCM envelope. Randomized ciphertext
  cannot be filtered or sorted; use a separate keyed blind index where needed.
  Encrypted and `#[orm(masked)]` values appear as `"***"` in generated
  `to_json()`, audit rows and committed events, are omitted from Scout
  documents, and stay encrypted in `save_to_redis` hashes. `SecretString`
  fields are redacted the same way, and `SecretString` serializes as an
  encrypted envelope rather than plaintext. `RULLST_ENCRYPTION_KEY` (and its
  `_KEY_ID`/`_KEYRING` companions) is read from the process environment, then
  from `./.env`; see [field encryption](../tutorials/15-rullst-vault-encryption.md).
- **Native relational enums:** `#[derive(Enum)]` owns one closed label mapping
  for SQLx, Serde and ORM values. `Blueprint::native_enum` emits a named,
  drift-checked PostgreSQL type with `strict-postgres`, inline MySQL/MariaDB
  `ENUM`, or a SQLite `TEXT CHECK` constraint.
- **Scout hooks and providers:** `#[orm(searchable)]` calls a configured
  `SearchEngine` after generated writes/deletes. `scout-http` supplies bounded
  Meilisearch, Elasticsearch and Algolia adapters; the generated effect is
  process-local unless the application composes the transactional outbox. See
  [Scout Search Providers](../tutorials/39-scout-search.md).
- **Typed pgvector queries:** `pgvector` re-exports `Vector` with SQLx support;
  vector/distance values in L2, cosine and inner-product helpers are bound, not
  interpolated. The strict PostgreSQL matrix creates the extension and runs a
  typed live lifecycle. See [RAG Systems & Vector Search](../tutorials/22-rag-vector-search.md).
- **Generated embeddings:** `ai` implies `pgvector` and adds
  `save_with_embedding(&rullst_ai::AiClient)` to models with an
  `#[orm(embedding_for = "...")]` field; the application must also depend on
  `rullst-ai`.
- **Bounded Qdrant vectors:** `qdrant` keeps specialized dense-cosine
  collection/upsert/delete/query semantics separate from SQL Active Record,
  with resource/transport bounds, deterministic fallback, authenticated
  protocol fixtures and a pinned live lifecycle.
- **Native Redis structures:** `redis` adds an immutable namespace and bounded
  Hash, Set and Sorted Set operations in addition to `.remember`; remote
  endpoints require TLS and live evidence covers isolation and native commands.
- **Portable document recovery:** MongoDB, SurrealDB and the deterministic
  store expose identifier-preserving inventory. An application-operated,
  AES-256-GCM snapshot binds application/collection scope, compares two bounded
  source observations, resumes only into an exact destination subset and
  verifies the final inventory. Writers, schema provisioning, key custody and
  durable backup storage remain explicit operator responsibilities. See
  [Polyglot Persistence](../polyglot-persistence.md).
- **Structured telemetry:** generated/raw query and stream spans expose only
  static model/table/operation metadata, managed transactions record bounded
  outcomes, and Rullst-created pools emit checkout timing. Core's opt-in
  OpenTelemetry layer can export the standard tracing signals; subscriber,
  sampling, collector and separately configured SQLx logs remain host policy.
- **Comparative SQLite evidence:** a lockfile-pinned Criterion harness gives
  Rullst, Diesel and SeaORM one typed connection, the same indexed schema,
  100-row seed, SQLite policy and five logical operations. The CI history is
  scoped comparison evidence; it does not claim universal or negligible
  overhead, networked-database throughput or complete-application performance.
- **Durable opt-in outbox:** `Outbox::enqueue` commits a stream-scoped,
  idempotent event with relational domain state. Exact lease tokens, bounded
  retry and dead-letter are shared by SQLite, PostgreSQL, MySQL and MariaDB,
  and streams and event keys are case-sensitive on all of them.
  Delivery is at least once, so the application dispatcher and consumer remain
  idempotent; generated observers are not silently converted into events.
  A nested `Orm::transaction` joins the active transaction through a
  savepoint, so a helper that enqueues inside its own transaction stays atomic
  with its caller. Concurrent sibling nested transactions take turns on the
  shared connection, and a savepoint left open makes the enclosing transaction
  roll back instead of committing. See
  the [transactional outbox tutorial](../tutorials/38-transactional-outbox.md).
- **Database-first introspection:** `cargo rullst generate:models` reads SQLite,
  PostgreSQL, or MySQL metadata using bound schema/table parameters, normalizes
  table module identifiers, and rejects unsafe SQL identifiers, collisions, or
  columns requiring unsupported ORM remapping before writing files.
- **Additive migration generation:** `make:migration:auto` compares supported
  model definitions and emits a migration for review.
- **Cascading soft deletes:** Opt-in relationship metadata can cascade through
  generated delete methods; transaction-aware variants use the supplied
  transaction. The related model must also use soft deletes; a cascade into a
  model without them fails to compile instead of hard-deleting its rows. The
  children are selected within the tenant scope but outside the related
  model's `global_scope`.
- **Transactional partial updates (v13 candidate):** `.update_partial()` merges
  selected values into a fresh row and performs its full save lifecycle, with
  explicit transaction support. Use it instead of `save()`, which writes the
  whole row, when requests may edit the same row at the same time; see
  [concurrent updates and counters](#concurrent-updates-and-counters) and
  [the contract and migration](../transactional-partial-updates.md).
- **Model policies:** `#[orm(policy = "MyPolicy")]` invokes the configured
  policy on generated create/update/delete/restore operations.
- **Strict lazy-loading prevention:** the global toggle makes generated lazy
  relationship methods return a validation error instead of performing the
  query.
- **Explicit Capability Boundaries**: Unsupported replication paths fail closed instead of reporting simulated success.

---

## 🛠️ Quick Start

### Installation

Install the v13 prerelease with an exact pin:

```bash
cargo add rullst-orm@=13.0.0-alpha.4 -F strict-sqlite
cargo add tokio -F full
```

Select the driver of your database: `strict-sqlite`, `strict-postgres` or
`strict-mysql`, or `drivers-all` for all three on SQLx's dynamic `Any` pool.
Since 13.0.0-alpha.3 the crate enables no driver by default; without one,
`Orm::init` fails at startup with an error that names the feature to enable.

### Zero-to-Hero Example

```rust,no_run
use rullst_orm::{Orm, FromRow};

// 1. Just add the Orm macro to your struct!
#[derive(Debug, Clone, FromRow, Orm)]
pub struct User {
    pub id: i32, // ID = 0 means it hasn't been saved yet
    pub name: String,
    pub email: String,
    #[orm(hidden)] // Won't be exposed in JSON responses
    pub password: String,
}

#[tokio::main]
async fn main() -> Result<(), rullst_orm::Error> {
    // 2. Initialize the connection pool (Supports SQLite, Postgres, MySQL)
    Orm::init("sqlite::memory:").await?;

    // 3. Create a new user
    let mut user = User {
        id: 0,
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
        password: "secret_password".to_string(),
    };
    
    user.save().await?; // Runs INSERT and hydrates the generated ID.

    // 4. Fluent Queries
    let active_users = User::query()
        .where_like("email", "%@example.com")
        .order_by_desc("id")
        .limit(10)
        .get()
        .await?;

    println!("Found users: {:?}", active_users);

    Ok(())
}
```

### Concurrent updates and counters

By default, `save()` on a loaded model writes every persisted column of that
handle, so the last writer wins: when two requests load the same row, change
different fields and save, the second `save()` silently reverts the first
one's change. An insert also sends every field, so database column defaults
(such as `DEFAULT CURRENT_TIMESTAMP`) do not apply to model fields, except to
fields marked `#[orm(db_default)]`.

**Dirty tracking (v13).** Declare one `#[sqlx(skip)]` field of type
`RowSnapshot`, and `save()` writes only the columns that this handle changed:

```rust,no_run
use rullst_orm::{FromRow, Orm, RowSnapshot};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "profiles")]
pub struct Profile {
    pub id: i32,
    pub name: String,
    pub bio: String,
    #[sqlx(skip)]
    pub snapshot: RowSnapshot,
}

async fn rename(id: i32, name: String) -> Result<(), rullst_orm::Error> {
    let mut profile = Profile::find(id)
        .await?
        .ok_or(rullst_orm::Error::RecordNotFound)?;
    profile.name = name;
    // `profile.dirty_columns()` is now `["name"]`.
    profile.save().await // UPDATE profiles SET name = ? WHERE id = ?
}
```

- Records read through the ORM (`find`, `first`, `get`, `all`, `paginate`,
  relations, streams, the query cache and the row `update_partial()` reads)
  remember a 64-bit fingerprint of every column that `save()` updates: 16
  bytes per record plus 8 per column, never the values. `save()` compares the
  current values with them: `#[orm(encrypted)]` and `SecretString` fields by
  plaintext, so an unchanged secret is not encrypted again, JSON fields
  (`#[sqlx(json)]`, `Json<T>`, `serde_json::Value`) by their canonical
  document, and other fields by their Serde serialization.
- The `UPDATE` sets only the changed columns, keeps the tenant predicate and
  returns the stored row (PostgreSQL and SQLite; MySQL/MariaDB read it again in
  the same transaction). The columns it did not write, `#[orm(db_default)]`
  columns and the soft-delete marker take their stored values, so the handle,
  the audit diff, model events and the search document describe the row as
  stored. A row deleted meanwhile still fails with `RecordNotFound`.
- When nothing changed, `save()` runs no `UPDATE`, no `updating`, `updated` or
  `saved` observers, no `after_save` hook, audit entry or post-commit event.
  The `before_save` hook and the `saving` observers run first and may change
  fields; a model with neither, and without a policy, does not even open a
  transaction.
- A record built in code holds `RowSnapshot::default()`: `save()` inserts it,
  or, with a non-zero `id`, writes every column. `mark_all_dirty()` makes a
  loaded record do the same, for code that relied on overwriting the row.
  `is_dirty()` tells whether `save()` has anything to write.
- A failed `save()` keeps its changes pending. When a caller-owned transaction
  rolls back after a successful `save()`, reload the record or call
  `mark_all_dirty()`.
- A `before_save` hook that stamps `updated_at` on every call makes every save
  write; stamp it only when `self.is_dirty()`.
- Mark the field `#[serde(skip)]` when the model derives Serde traits. Any two
  snapshots compare equal and hash alike, so a derived `PartialEq` or `Hash`
  looks at the columns only. Since 13.0.0-alpha.3 every SQLx model that the
  CLI writes declares the field: `make:model` (and `make:resource`),
  `generate:models` (with `#[serde(skip)]`; a table with its own `snapshot`
  column keeps full-row saves), `make:auth`, `make:billing`,
  `make:chat-session` and the models of every `cargo rullst new` starter
  (Blank, API and SaaS) and of the LMS, ERP and Portfolio examples. Their struct literals set
  `snapshot: RowSnapshot::default()`. Turso models have no dirty tracking.
  Projects generated earlier keep their models until you add the field.

Dirty tracking merges edits of different columns. Two writers of the same
column still race, and the last one wins:

- To change some fields of a row that other requests may edit, use
  `update_partial()`. It reads the current row in a transaction (with
  `SELECT ... FOR UPDATE` on PostgreSQL and MySQL/MariaDB), sets only the
  fields you chose and saves through the normal policy, hooks, audit and cache
  lifecycle. On SQLite a contended `update_partial()` can fail with a database
  error instead of waiting; retry it. It merges fields, but it does not detect
  two editors changing the same field: that needs a revision column checked by
  the application.
- For a counter, run one atomic statement with `Orm::raw`. Its values are
  bound and it joins the active `Orm::transaction`, but it skips the model's
  tenant scope, policy, hooks, audit and cache invalidation: add the tenant
  condition yourself. Raw SQL uses the driver's own markers, `$1`, `$2` on
  PostgreSQL and `?` on SQLite and MySQL/MariaDB.

```rust,no_run
use rullst_orm::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "students")]
pub struct Student {
    pub id: i32,
    pub bio: String,
    pub xp: i32,
}

/// Changes only `bio`: an XP award committed meanwhile is kept.
async fn edit_bio(student: &mut Student, bio: String) -> Result<(), rullst_orm::Error> {
    student.update_partial().bio(bio).save().await
}

/// Adds XP in one statement, so concurrent awards add up.
async fn award_xp(student_id: i32, points: i32) -> Result<(), rullst_orm::Error> {
    let sql = if Orm::driver()? == "postgres" {
        "UPDATE students SET xp = xp + $1 WHERE id = $2"
    } else {
        "UPDATE students SET xp = xp + ? WHERE id = ?"
    };
    let changed = Orm::raw(sql).bind(points).bind(student_id).execute().await?;
    if changed == 0 {
        return Err(rullst_orm::Error::RecordNotFound);
    }
    Ok(())
}
```

### Database errors

A unique or primary-key violation returns `Error::UniqueViolation`, and a
foreign-key violation (a missing referenced row, or deleting a row that is
still referenced) returns `Error::ForeignKeyViolation`. Generated writes, query
builders and `Orm::raw` all map them; every other database failure stays
`Error::DatabaseError(String)`. `constraint` holds the name PostgreSQL reports
and is `None` on SQLite and MySQL/MariaDB. The message of these two errors
names only the constraint, because the database's own text can quote the
rejected value (MySQL reports `Duplicate entry 'alice@example.com'`).

```rust,no_run
use rullst_orm::{Error, FromRow, Orm};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "members")]
pub struct Member {
    pub id: i32,
    pub email: String,
}

/// `Ok(None)` when the email is already registered.
async fn register(email: String) -> Result<Option<Member>, Error> {
    let mut member = Member { id: 0, email };
    match member.save().await {
        Ok(()) => Ok(Some(member)),
        Err(Error::UniqueViolation { .. }) => Ok(None),
        Err(error) => Err(error),
    }
}
```

`Orm::transaction` returns its closure's error unchanged after the rollback,
so a violation inside it keeps its variant too. The closure may return the
application's own error type when it implements `From<rullst_orm::Error>` and
`Display`; failures of the transaction itself, such as a commit rejected by a
deferred constraint, arrive through that conversion.

### Query values

Filters such as `where_eq` and `where_in`, `Orm::raw(..).bind(..)` and
`with_tenant` take any value that converts into `RullstValue`: `String`/`&str`,
`i32`, `i64`, `u32`, `f64`, `bool`, the `chrono` types `DateTime<Utc>`,
`NaiveDateTime`, `NaiveDate` and `NaiveTime` (see
[date and time columns](#date-and-time-columns)) and `Option` of those. `None` binds SQL
`NULL`, typed as text: on PostgreSQL write `CAST($1 AS BIGINT)` to store it in
a column of another type. A comparison with `NULL` never matches, so filter
missing values with `where_null`. `RullstValue` is `#[non_exhaustive]`; give a
`match` on it a wildcard arm.

```rust,no_run
use rullst_orm::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "progress")]
pub struct Progress {
    pub id: i32,
    pub lesson_id: Option<i64>,
    pub seconds: i64,
}

async fn for_lesson(lesson_id: Option<i64>) -> Result<Vec<Progress>, rullst_orm::Error> {
    let query = Progress::query();
    match lesson_id {
        Some(lesson_id) => query.where_eq("lesson_id", lesson_id).get().await,
        None => query.where_null("lesson_id").get().await,
    }
}
```

### Query row cap

Generated builders start with a global row cap (`Orm::set_max_query_limit`,
1,000 by default; `0` disables it). `limit()` clamps to that cap and
`unsafe_unlimited()` removes it for one explicit query. `paginate(page,
per_page)` clamps `per_page` to the same cap, because the value often comes
from request input; `PaginationResult::per_page` and `last_page` report the
effective page size. `count()` and the `paginate()` total count the rows the
query returns: a `distinct()` or `group_by()` query is counted as a derived
table, so the total is the number of distinct rows or groups.

Eager loading runs one related-model query for all parents of a batch and
never assigns relations from a result truncated by that cap: when the related
rows exceed it, `get()` fails with a `Validation` error naming the relation.
Load fewer parents per query, raise the cap, or choose explicitly with
`with_<relation>_constrained(...)`: an explicit smaller `limit(n)` there applies
to the whole batch, and `unsafe_unlimited()` loads every related row.

Parents that share a related row or group all receive it: every child of one
`belongs_to` parent, parents whose non-unique `local_key` matches the same
`has_many`/`has_one` rows, and duplicated parent rows. The shared value is
cloned for all but the last such parent, so a related model without `Clone`
loads normally until a row must be shared, and then `get()` fails with a
`Validation` error instead of leaving a parent without its relation.

A relation key may be a nullable foreign key (`author_id: Option<i32>`) on
either model. A `None` key matches no row, like SQL `NULL`: the lazy loader
returns `None` or an empty list without a query, an eager load gives such a
parent `None` (or `Some(vec![])` for a to-many relation), and a related row
whose key is `None` belongs to no parent.

### Native database enums

Generated applications should select a strict primary feature. PostgreSQL
native enums specifically require `strict-postgres`, because SQLx's dynamic
`Any` driver cannot decode custom PostgreSQL types:

```toml
rullst-orm = { version = "=13.0.0-alpha.4", features = ["strict-postgres"] }
```

Derive one label contract and use it in schema code:

```rust
use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{Enum, Orm};

#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq)]
#[rullst_enum(type_name = "account_status", rename_all = "snake_case")]
enum AccountStatus {
    AwaitingReview,
    Active,
}

# async fn create_schema() -> Result<(), rullst_orm::Error> {
Orm::init("postgres://user:password@localhost/application").await?;
Schema::create("accounts", |table: &mut Blueprint| {
    table.id();
    table.native_enum::<AccountStatus>("status").not_null();
}).await?;
# Ok(())
# }
```

The derive accepts 1–64 unique labels of at most 63 bytes using ASCII letters,
digits, spaces, underscores or hyphens. An existing PostgreSQL type must have
the exact same ordered labels or schema creation fails. MySQL/MariaDB store the
labels in the table's inline `ENUM`; SQLite enforces them through `TEXT CHECK`.
Adding, removing or reordering labels is an explicit reviewed migration. Drop
every dependent table before calling `Schema::drop_native_enum::<T>()` on
PostgreSQL; the method is a validated no-op on the other backends. The enum
type creation, its label check and `drop_native_enum` use the active
`Orm::transaction` or test sandbox like the table DDL, so they roll back with it
and the type can be dropped right after its tables in the same transaction.

Builder filters on a model field whose type derives `Enum` (or
`Option<...>` of it) work on every backend: on PostgreSQL the comparison,
`IN` and `BETWEEN` markers of that column become `CAST(? AS "<type_name>")`,
because a text parameter has no operator against a named enum type. This
covers `where_eq`, `where_in`, the generated `where_<column>` helpers and
their `or_`/`not_` variants; `where_like` and raw SQL are unchanged.

### Constraints, indexes and foreign keys

`Schema::create` declares unique constraints, indexes, foreign keys and CHECK
constraints next to the columns. Every name is validated with the identifier
grammar before it reaches DDL:

```rust,no_run
use rullst_orm::schema::{Blueprint, ColumnDefault, ForeignKeyAction, Schema};

# async fn create_schema() -> Result<(), rullst_orm::Error> {
Schema::create("enrollments", |table: &mut Blueprint| {
    table.id();
    // An `INTEGER NOT NULL` column referencing `users(id)`.
    table
        .foreign_id("user_id", "users")
        .on_delete(ForeignKeyAction::Cascade);
    table.integer("course_id").not_null();
    table
        .foreign("course_id")
        .references("courses", "id")
        .on_delete(ForeignKeyAction::Restrict);
    // MySQL/MariaDB cannot index a TEXT column: give indexed strings a length.
    table.string("code").not_null().unique().col_type = "VARCHAR(64)".to_string();
    table.integer("progress").not_null().default(ColumnDefault::Integer(0));
    table.unique(&["user_id", "course_id"]);
    table.index(&["course_id", "progress"]).name("enrollments_course_progress");
    table.check("progress BETWEEN 0 AND 100");
    table.timestamps();
}).await?;
# Ok(())
# }
```

- `Column::unique()` adds a column-level `UNIQUE`; `unique(&[...])` a table
  constraint named `<table>_<columns>_unique` unless `.name(...)` sets one.
- `index(&[...])` and `unique_index(&[...])` are `CREATE [UNIQUE] INDEX IF NOT
  EXISTS <table>_<columns>_index|_unique` statements after `CREATE TABLE` on
  PostgreSQL and SQLite, and `INDEX` clauses of `CREATE TABLE` on
  MySQL/MariaDB (which has no `IF NOT EXISTS` for indexes). Names are at most
  63 bytes, so PostgreSQL never truncates them; a longer default name fails and
  asks for `.name(...)`.
- `foreign(column).references(table, column)` adds `<table>_<column>_foreign`
  with optional `on_delete`/`on_update` (`Cascade`, `Restrict`, `SetNull`,
  `NoAction`). `foreign_id(column, table)` also adds the `INTEGER NOT NULL`
  column and references `table(id)`. The column type must match the referenced
  one (`id()` is `INTEGER`), `SetNull` needs a nullable column, and SQLite
  enforces foreign keys only with `PRAGMA foreign_keys = ON`, which SQLx sets
  on every connection it opens.
- `check(expression)` adds `CHECK (expression)`. The expression is trusted
  developer SQL emitted as written: building fails when it is empty, longer
  than 512 bytes, or contains `;`, a comment, `\`, `$`, a backtick, a control
  character or unbalanced parentheses or quotes, but never build it from
  request data. MySQL enforces CHECK from 8.0.16 and MariaDB from 10.2.1.
- On MySQL/MariaDB a unique, index or foreign-key column whose type is `TEXT`,
  `BLOB` or `JSON` (a plain `string()`, or `timestamps()` on the `Any` pool)
  fails before any DDL runs: set a bounded `col_type` such as `VARCHAR(255)`.

A unique or foreign-key violation from a later write returns
`Error::UniqueViolation`/`Error::ForeignKeyViolation`; PostgreSQL reports the
constraint name.

`Schema::table` alters an existing table with the same builders: it adds the
blueprint's columns (`ALTER TABLE ... ADD COLUMN`), then its unique
constraints, indexes, foreign keys and checks, all validated before the first
statement runs:

```rust,no_run
use rullst_orm::schema::{Blueprint, ForeignKeyAction, Schema};

# async fn add_reviewer() -> Result<(), rullst_orm::Error> {
Schema::table("posts", |table: &mut Blueprint| {
    table.integer("reviewer_id");
    table
        .foreign("reviewer_id")
        .references("users", "id")
        .on_delete(ForeignKeyAction::SetNull);
    table.index(&["reviewer_id"]);
}).await?;
# Ok(())
# }
```

SQLite cannot add a constraint to an existing table. There a unique
constraint, or a `unique()` column, becomes the equivalent unique index, and a
foreign key on a column added in the same call becomes that column's
`REFERENCES` clause (the column must be nullable without a default). Any other
foreign key (on an existing column, or a second one on the same new column), a CHECK, a primary-key column, a `NOT NULL` column without a
default or a current-time default returns `Error::UnsupportedSchemaChange`
before any statement runs; rebuild such a table with a new `Schema::create`.
MySQL/MariaDB commit each DDL statement on its own, so a failure there can leave
the earlier statements applied.

### Date and time columns

`date()`, `time()`, `timestamp()` and `timestamp_tz()` hold `chrono` values
(re-exported as `rullst_orm::chrono`, and in the facade as
`rullst::orm::chrono`):

| Helper | Rust field | PostgreSQL | MySQL/MariaDB | SQLite (ISO-8601 text) |
| :--- | :--- | :--- | :--- | :--- |
| `date` | `NaiveDate` | `DATE` | `DATE` | `2026-10-15` |
| `time` | `NaiveTime` | `TIME` | `TIME(6)` | `09:30:00.500` |
| `timestamp` | `NaiveDateTime` | `TIMESTAMP` | `DATETIME(6)` | `2026-10-15 09:30:00.500` |
| `timestamp_tz` | `DateTime<Utc>` | `TIMESTAMPTZ` | `DATETIME(6)` (UTC) | `2026-10-15T09:30:00.500+00:00` |

MySQL/MariaDB store a UTC instant in `DATETIME(6)` because their `TIMESTAMP`
ends in 2038; SQLx sets the session time zone to `+00:00`, so values and
`CURRENT_TIMESTAMP` defaults are UTC (other clients see the same UTC wall
time). A `ColumnDefault::CurrentTimestamp` default is the current UTC time in
the column's own form: `CURRENT_TIMESTAMP` on a PostgreSQL `TIMESTAMPTZ`, the
UTC wall time (`CURRENT_TIMESTAMP AT TIME ZONE 'UTC'`) on the zone-less types,
`CURRENT_TIMESTAMP(6)`, `(CURRENT_DATE)` or `(CURRENT_TIME(6))` on
MySQL/MariaDB, and on SQLite the same text SQLx writes, so stored values of one
column compare and sort as text (`strftime('%Y-%m-%dT%H:%M:%f+00:00', 'now')`
for `timestamp_tz`).

`table.timestamps()` adds nullable `created_at`/`updated_at` `timestamp_tz()`
columns that default to the current time. Model fields, filters and ordering
use the `chrono` types; `RullstValue` converts them (`where_gt("starts_at",
Utc::now())`), and `#[orm(db_default)]` leaves a column to its database
default:

```rust,ignore
// Needs a strict driver feature (see below).
use rullst_orm::chrono::{DateTime, NaiveDate, Utc};
use rullst_orm::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "events")]
pub struct Event {
    pub id: i32,
    pub starts_at: DateTime<Utc>,
    pub day: NaiveDate,
    pub ends_at: Option<DateTime<Utc>>,
    // Never written by `save()`: INSERT keeps the column DEFAULT and UPDATE
    // keeps the stored value. The handle keeps its own value (here `None`)
    // until the row is read again.
    #[orm(db_default)]
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}
```

`#[orm(db_default)]` fields are left out of the generated `INSERT` and of
every `UPDATE` of `save()`, `update_partial()` has no setter for them and
`#[derive(Nexus)]` shows them read-only. Set an `updated_at` without it
yourself (`Utc::now()`) or maintain it with a trigger. A model with a
`RowSnapshot` field refreshes them from the stored row after each update.

`#[orm(db_default)]` is also the option for any column that the database or
your own SQL manages, such as the generated `users.is_admin` and
`users.session_epoch`: queries can filter, order, group and select it, and
ORM writes never touch it. `#[orm(skip)]` means something else: the field is
not a database column for the ORM, so a filter, ordering, grouping or
`select` on it fails with `Error::Validation`. A skipped field that the table
does not have also needs `#[sqlx(skip)]` (or `#[sqlx(default)]`), because the
application's SQLx `FromRow` still reads `#[orm(skip)]` fields.

| Option | The column | Filters, ordering, grouping, `select` | ORM writes |
| --- | --- | --- | --- |
| `#[orm(skip)]` | is not a database column for the ORM | rejected | never |
| `#[orm(db_default)]` | exists and is managed by the database | allowed | never |

Date/time columns and `chrono` fields need a strict driver feature
(`strict-postgres`, `strict-mysql` or `strict-sqlite`, which generated projects
use). SQLx's dynamic `Any` pool, selected by `drivers-all` without a
strict feature, carries only integer, float, text and blob values: it cannot
decode a `chrono` field, and it cannot read a PostgreSQL or MySQL/MariaDB
`DATE`, `TIME`, `TIMESTAMP` or `TIMESTAMPTZ` column at all. On that pool
`Schema::create` and `Schema::table` reject those helpers on PostgreSQL and
MySQL/MariaDB with `Error::Validation`, SQLite keeps its ISO-8601 text for
`String` fields, chrono filter values bind as that same text, and
`timestamps()` keeps the alpha.1 `TEXT DEFAULT CURRENT_TIMESTAMP` columns
(`DEFAULT (CURRENT_TIMESTAMP)` on MySQL/MariaDB, which accept a default on
`TEXT`, `BLOB`, `JSON` and `GEOMETRY` columns only as an expression; other
non-`NULL` defaults on those types are wrapped the same way, MySQL 8.0.13+,
MariaDB 10.2.1+). `soft_deletes()` stays a `TEXT` `deleted_at` column on every
pool; declare `timestamp_tz("deleted_at")` for a native one. The Turso adapter
does not use the Blueprint and keeps text timestamps.

`table.float(...)` and `table.boolean(...)` map to `f64` and `bool` model
fields. PostgreSQL receives `DOUBLE PRECISION` and `BOOLEAN` (an integer
`ColumnDefault` of `0`/`1` becomes `FALSE`/`TRUE`); MySQL/MariaDB receive
`DOUBLE` and an `INTEGER` 0/1 flag; SQLite keeps `REAL` and `INTEGER`. This
applies to newly built DDL only: PostgreSQL columns created by earlier versions
remain `REAL`/`INTEGER` until a reviewed migration alters them, and a model that
paired `boolean()` with an integer field on PostgreSQL must switch to `bool`
(or use `integer()`) for new tables.

---

## 📚 Documentation

We recently launched a brand-new **Interactive Documentation Hub**! 

👉 **[Explore the Full Documentation in the Rullst Book](https://rullst.github.io/Rullst/book/)**

---

## 🛡️ Security

Rullst ORM uses SQLx prepared-statement bindings for values accepted by its query builders. Structural identifiers are restricted to a bounded ASCII identifier grammar before interpolation. Raw SQL and application authorization remain the caller's responsibility; these controls reduce injection risk but are not an absolute safety guarantee.

## 📄 License
This project is licensed under the [MIT License](../../../LICENSE).
