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

🚀 **[Visit the Official Website & Documentation Hub](https://rullst.github.io/book/)** 🚀

Built on top of `sqlx` and procedural macros, **Rullst ORM** brings the delightful, fluent syntax of Active Record frameworks directly to the high-performance Rust ecosystem.

<div align="center">
  <h3>🛡️ Security Engineering</h3>
  <p>Rullst ORM uses SQLx bindings, validated identifiers, typed errors, and layered CI checks. Workflow badges are scoped test results, not a guarantee for an application or deployment.</p>

| Security Audit | Status | Description |
| :--- | :---: | :--- |
| **OpenSSF Scorecard** | <a href="https://scorecard.dev/viewer/?uri=github.com/Rullst/Rullst"><img src="https://img.shields.io/badge/dynamic/json?url=https%3A%2F%2Fapi.scorecard.dev%2Fprojects%2Fgithub.com%2FRullst%2FRullst&query=%24.score&label=OpenSSF%20Scorecard&style=flat-square" alt="OpenSSF Scorecard" /></a> | Current public supply-chain practice score; not a security certification |
| **Release Provenance** | <a href="https://github.com/Rullst/Rullst/actions/workflows/release.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/release.yml?style=flat-square&label=" alt="Release provenance" /></a> | Provenance attestations for release artifacts; no SLSA level is claimed here |
| **Codecov** | <a href="https://codecov.io/gh/Rullst/Rullst"><img src="https://codecov.io/github/Rullst/Rullst/branch/main/graph/badge.svg?component=framework_libraries" alt="Framework library coverage" /></a> | Blocking 90% target for the measured framework-library scope; repository aggregate remains separately visible |
| **Matrix DB Tests** | <a href="https://github.com/Rullst/Rullst/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/ci.yml?style=flat-square&label=" alt="Testcontainers" /></a> | Dockerized PostgreSQL & MySQL integration tests |
| **OpenSSF** | <a href="https://www.bestpractices.dev/projects/13359"><img src="https://img.shields.io/cii/level/13359?style=flat-square&label=" alt="OpenSSF Best Practices" /></a> | Open source security standards |
| **Property tests** | <a href="https://github.com/Rullst/Rullst/actions/workflows/proptest.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/proptest.yml?branch=main&style=flat-square&label=Proptest" alt="Proptest" /></a> | Scheduled/manual bounded invariant evidence |
| **Miri research matrix** | <a href="https://github.com/Rullst/Rullst/actions/workflows/miri.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/miri.yml?branch=main&style=flat-square&label=Miri" alt="Miri" /></a> | Manual, partly non-blocking evidence for the targets actually exercised |
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
  supported model shapes.
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
  instance saves/deletes and their audit entry share a savepoint and fail
  together. Eligible v2 updates expose guarded revision restoration, which
  rejects stale, cross-tenant, redacted, malformed, legacy, create/delete, and
  oversized revisions and records a compensating audit entry. The host still
  derives authenticated principal/tenant authority, while bulk per-row history
  and durable export remain explicit. See
  [Auditable Revisions](../tutorials/50-auditable-revisions.md).
- **Field privacy:** `#[orm(encrypted)]` transparently encrypts supported
  `String` fields with a versioned AES-256-GCM envelope. Randomized ciphertext
  cannot be filtered or sorted; use a separate keyed blind index where needed.
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
  retry and dead-letter are shared by SQLite, PostgreSQL, MySQL and MariaDB.
  Delivery is at least once, so the application dispatcher and consumer remain
  idempotent; generated observers are not silently converted into events. See
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

Add the library to your `Cargo.toml`:

```bash
cargo add rullst-orm
cargo add tokio -F full
```

### Zero-to-Hero Example

```rust
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
    
    user.save().await?; // Runs INSERT and hydrates the ID automatically!

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

---

## 📚 Documentation

We recently launched a brand-new **Interactive Documentation Hub**! 

👉 **[Explore the Full Documentation in the Rullst Book](https://rullst.github.io/book/)**

---

## 🛡️ Security

Rullst ORM uses SQLx prepared-statement bindings for values accepted by its query builders. Structural identifiers are restricted to a bounded ASCII identifier grammar before interpolation. Raw SQL and application authorization remain the caller's responsibility; these controls reduce injection risk but are not an absolute safety guarantee.

## 📄 License
This project is licensed under the [MIT License](../../../LICENSE).
