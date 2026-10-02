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
> This page targets `12.1.0`. Check the [release record](../v12.md) for
> publication status; use a path dependency only for checkout-local review.

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
generated API.

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
  encrypted envelope rather than plaintext.
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
  transaction.
- **Partial updates:** `.update_partial()` binds only the selected supported
  fields.
- **Model policies:** `#[orm(policy = "MyPolicy")]` invokes the configured
  policy on generated create/update/delete/restore operations.
- **Strict lazy-loading prevention:** the global toggle makes generated lazy
  relationship methods return a validation error instead of performing the
  query.
- **Explicit Capability Boundaries**: Unsupported replication paths fail closed instead of reporting simulated success.

---

## 🛠️ Quick Start

### Installation

After crates.io indexes the RC, install its exact train with:

```bash
cargo add rullst-orm@12.1.0
cargo add tokio -F full
```

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

### Query row cap

Generated builders start with a global row cap (`Orm::set_max_query_limit`,
1,000 by default; `0` disables it). `limit()` clamps to that cap and
`unsafe_unlimited()` removes it for one explicit query. `paginate(page,
per_page)` clamps `per_page` to the same cap, because the value often comes
from request input; `PaginationResult::per_page` and `last_page` report the
effective page size.

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

### Native database enums

Generated applications should select a strict primary feature. PostgreSQL
native enums specifically require `strict-postgres`, because SQLx's dynamic
`Any` driver cannot decode custom PostgreSQL types:

```toml
rullst-orm = { version = "12.1.0", features = ["strict-postgres"] }
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
`Option<...>` of it) work on every backend: under the `strict-postgres`
runtime the comparison, `IN` and `BETWEEN` markers of that column become
`CAST(? AS "<type_name>")`, because a text parameter has no operator against a
named enum type. This covers `where_eq`, `where_in`, the generated
`where_<column>` helpers and their `or_`/`not_` variants; `where_like` and raw
SQL are unchanged. SQLx `Any` cannot decode named enum types, so on PostgreSQL
through `Any` such fields stay text columns and keep plain `?` markers.

`table.timestamps()` adds nullable `created_at`/`updated_at` `TEXT` columns
that default to the current timestamp. MySQL/MariaDB reject a literal default
on `TEXT`, `BLOB`, `JSON` and `GEOMETRY` columns, so on that driver the
builder emits `DEFAULT (CURRENT_TIMESTAMP)` and wraps other non-`NULL`
defaults on those types in parentheses (MySQL 8.0.13+, MariaDB 10.2.1+).
SQLite and PostgreSQL DDL is unchanged.

### Behaviour changes in 12.2

#### Upgrading from 12.1

The 12.2 ORM keeps the 12.x API, but an upgrading application can notice the
following. Check each item that applies before deploying:

- [ ] **Nested `Orm::transaction`** joins the outer transaction through a
  savepoint, so inner work rolls back with the outer one: check helpers that
  relied on committing on their own.
- [ ] **Sibling nested transactions** started together (for example with
  `tokio::join!`) take turns, and a savepoint left open rolls the outer
  transaction back with an error: check concurrent nested calls.
- [ ] **`SecretString` serializes as an encrypted envelope** (and fails without
  a configured key): check JSON responses or exports that included it; use
  `reveal_audited()` where plaintext is intended.
- [ ] **`paginate()` caps `per_page`** at the query limit (1,000 by default):
  check clients that request larger pages and read `per_page`/`last_page`.
- [ ] **Query-cache keys move to `rullst:orm:cache:v4:`**: expect a cold cache
  after deploying, and short TTLs during a rolling upgrade.
- [ ] **Redis model hashes are namespaced**: global models migrate lazily on
  their next write; tenant models need the procedure under
  "12.1 Redis model hashes" below.
- [ ] **12.1 Redis hashes with `#[orm(encrypted)]` fields** hold plaintext and
  fail closed on read: re-save them with `save_to_redis()`.
- [ ] **New typed errors for misuse**: `delete_all()` with `limit()`,
  `offset()`, `order_by()`, joins, grouping or CTEs; `only_trashed()` without
  soft deletes; a tenant context of the wrong type; raw CTE/select bind
  markers with scope, JOIN or WHERE bindings; eager loads past the query limit
  or sharing a non-`Clone` related row; a savepoint left open. Check logs and
  tests for these errors.
- [ ] **`restore()` and `force_delete()`** run hooks, observers, audit and
  post-commit effects: a `before_delete` veto now blocks `force_delete()`.
- [ ] **Secondary projections** (`to_json()`, audit rows, `orm:events:*`) carry
  `"***"` for encrypted and masked fields, and Scout documents omit them: check
  consumers of those payloads.
- [ ] **`search()` without a search engine** skips hidden, encrypted, masked and
  `SecretString` columns and matches `%`/`_` literally: check searches that
  relied on them.
- [ ] **Tenant-scoped `search()`** answers a 1,000-hit engine result from the
  SQL fallback; other models keep the engine answer.
- [ ] **New DDL only**: MySQL/MariaDB audit payloads become `LONGTEXT` and
  `float()` becomes double precision in newly created tables; existing tables
  keep their types (watch for the audit-table warning and migrate if needed).
- [ ] **Enum filter casts** apply only under `strict-postgres`; nothing changes
  on SQLx `Any`.
- [ ] **`#[derive(Nexus)]`** hides encrypted, `SecretString` and
  `#[orm(hidden)]` fields and omits skipped ones: check admin workflows that
  edited them.
- [ ] **Generated Redis code follows `rullst-orm/redis`** (or `rullst`'s
  `redis`/`orm-redis`): applications that call `Orm::init_redis*` without a
  `redis` feature of their own now invalidate the cache and publish
  `orm:events:*` on every generated write.
- [ ] **Stale soft-delete handles** are unchanged from 12.1: saving a handle
  loaded before `delete()` writes its old soft-delete value back and undeletes
  the row, so reload a model before saving it.

#### Details

- **Transactions:** concurrent sibling nested `Orm::transaction` calls take
  turns on the shared connection; a savepoint left open makes the enclosing
  transaction roll back and return an error, and the pool closes a connection
  returned while still inside a transaction.
- **Outbox:** MySQL/MariaDB read an idempotent duplicate back with
  `FOR UPDATE`; a row that still cannot be read back is `DatabaseError`, not
  `RecordNotFound`.
- **Schema:** new MySQL/MariaDB audit tables use `LONGTEXT` payload columns
  (existing tables log a warning naming the migration); `float()` emits
  `DOUBLE PRECISION`/`DOUBLE` on PostgreSQL/MySQL for new DDL; PostgreSQL enum
  DDL joins the task-scoped transaction. `boolean()` stays an `INTEGER` flag.
- **Generated Redis code** follows `rullst-orm/redis` (or the facade's
  `redis`/`orm-redis`): `.remember(...)`, commit-time invalidation and the
  `orm:events:*` publications appear without an application `redis` feature.
  Query-cache keys move to `rullst:orm:cache:v4:` with a per-table index, so
  caches start cold. Model hashes use namespaced keys, and tenant models
  require `with_tenant(...)`.
- **12.1 Redis model hashes** (`orm:<table>:<id>`): for a model without a
  tenant scope, `get_from_redis` reads the 12.1 hash while the namespaced one
  is missing, and the next `save_to_redis` or `increment_redis_field` moves it
  to the namespaced key; nothing has to be run. Applications that share one
  Redis database also shared these keys, so the first one to write a hash
  takes it over. A 12.1 hash of a model with `#[orm(encrypted)]` fields holds
  them in plaintext, so reading it fails closed until `save_to_redis()`
  rewrites it. Tenant models never read the 12.1 key, which every tenant
  shared, so until migrated `get_from_redis` returns `None` and
  `increment_redis_field` starts from zero. Migrate them once after deploying
  12.2:
  1. List the keys of each tenant model table:
     `redis-cli --scan --pattern 'orm:<table>:*'`.
  2. Read each hash with `HGETALL`; every value is the JSON of one field.
  3. Check its tenant column against the tenant that owns row `<id>` in the
     database, and skip mismatches: another tenant may have overwritten it.
  4. Decode the hash into the model (for example with `serde_json` from the
     parsed values), or reload the row when the hash only cached it, and call
     `with_tenant(tenant, model.save_to_redis())`.
  5. Remove the 12.1 key with `UNLINK`.
- **Queries:** eager loads give a shared related row to every parent;
  `delete_all()` rejects `limit()`, `offset()`, `order_by()`, joins, grouping
  and CTEs; `only_trashed()` fails on models without soft deletes; `query()`
  rejects a tenant context of the wrong type; raw CTE/select fragments with
  bind markers fail once scope, JOIN or WHERE bindings exist; strict PostgreSQL
  enum filters cast to the enum type; tenant-scoped `search()` answers a
  1,000-hit engine result from SQL.
- **Soft deletes:** `save()` still writes the soft-delete column from the
  handle, as in 12.1. A handle loaded before `delete()` (or before another
  request deleted the row) therefore restores the row when saved, without
  `can_restore`, the `restored` audit entry or restore observers. Reload the
  model before saving it, and change the marker only through
  `delete()`/`restore()`.
- **Audit and Nexus:** restore patches withhold sensitive keys that were added
  or removed; `#[derive(Nexus)]` omits skipped fields and keeps encrypted,
  `SecretString` and `#[orm(hidden)]` fields hidden and read-only.

---

## 📚 Documentation

We recently launched a brand-new **Interactive Documentation Hub**! 

👉 **[Explore the Full Documentation in the Rullst Book](https://rullst.github.io/Rullst/book/)**

---

## 🛡️ Security

Rullst ORM uses SQLx prepared-statement bindings for values accepted by its query builders. Structural identifiers are restricted to a bounded ASCII identifier grammar before interpolation. Raw SQL and application authorization remain the caller's responsibility; these controls reduce injection risk but are not an absolute safety guarantee.

## 📄 License
This project is licensed under the [MIT License](../../../LICENSE).
