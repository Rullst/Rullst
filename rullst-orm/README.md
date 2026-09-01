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

> **v12 development notice:** This README documents the unreleased v12 source.
> Use a path dependency from this checkout until an immutable v12 RC exists on
> crates.io. `12.0.0-rc.1` below is the planned first RC.

🚀 **[Visit the Official Website & Documentation Hub](https://rullst.github.io/Rullst/book/)** 🚀

Built on top of `sqlx` and procedural macros, **Rullst ORM** brings the delightful, fluent syntax of Active Record frameworks directly to the high-performance Rust ecosystem.

<div align="center">
  <h3>🛡️ Security Engineering</h3>
  <p>Rullst ORM uses SQLx bindings, validated identifiers, typed errors, and layered CI checks. Workflow badges are scoped test results, not a guarantee for an application or deployment.</p>

| Security Audit | Status | Description |
| :--- | :---: | :--- |
| **OpenSSF Scorecard** | <a href="https://scorecard.dev/viewer/?uri=github.com/Rullst/Rullst"><img src="https://img.shields.io/badge/dynamic/json?url=https%3A%2F%2Fapi.scorecard.dev%2Fprojects%2Fgithub.com%2FRullst%2FRullst&query=%24.score&label=OpenSSF%20Scorecard&style=flat-square" alt="OpenSSF Scorecard" /></a> | Current public supply-chain practice score; not a security certification |
| **Release Provenance** | <a href="https://github.com/Rullst/Rullst/actions/workflows/release.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/release.yml?style=flat-square&label=" alt="Release provenance" /></a> | Provenance attestations for release artifacts; no SLSA level is claimed here |
| **Codecov** | <a href="https://codecov.io/gh/Rullst/Rullst"><img src="https://codecov.io/github/Rullst/Rullst/branch/main/graph/badge.svg?component=framework_libraries" alt="Framework library coverage" /></a> | Blocking 90% target for the measured framework-library scope; repository aggregate remains separately visible |
| **Matrix DB Tests** | <a href="https://github.com/Rullst/Rullst/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/ci.yml?style=flat-square&label=" alt="Testcontainers" /></a> | Live PostgreSQL, MySQL, MariaDB, MongoDB, SurrealDB and libSQL contracts, plus in-process DuckDB tests |
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

In traditional Rust database handling, you have to write raw SQL queries, manage connection pools manually, and bind variables repetitively. Rullst ORM abstracts the heavy lifting behind a single `#[derive(Orm)]` macro, generating hundreds of safe, chainable query methods at compile time.

**Key Features:**
- **Generated CRUD**: Derive insert, update, delete, and find helpers from model metadata.
- **Typed Fluent Query Builder**: Generated primitive filters such as
  `.where_id(i32)` and `.where_name(impl Into<String>)` reject mismatched value
  types at compile time; dynamic string-column methods remain explicit escape
  hatches.
- **Eager Loading**: Batch supported `has_many`, `belongs_to`, `morph_many`,
  `morph_one`, and typed `morph_to` targets. Inverse polymorphic fields use an
  explicit target per relation and a persisted `<morph_name>_id` plus
  `<morph_name>_type` discriminator.
- **Fail-Closed Tenant Scopes**: Models declaring `tenant_column` require
  `with_tenant`, inject the tenant predicate into generated queries, protect
  instance mutations, and reserve explicit `unscoped()` for reviewed global
  paths. Authentication and permission to use that escape hatch remain the
  application's responsibility.
- **Actor-Bound Audit Revisions**: Opt-in auditable models require a validated
  user/service/system `AuditContext`, derive tenant metadata from `with_tenant`,
  and persist recursively redacted bounded changes in the model transaction.
  Eligible v2 update revisions can be restored with stale-state, tenant, model,
  identity, and sensitive-field guards; the compensating update records its
  actor, reason, and source revision. Bulk per-row history and durable external
  export remain explicit application/outbox work. See the
  [audit revision guide](../docs/src/tutorials/50-auditable-revisions.md).
- **Bounded Post-Commit Effects**: `after_commit` and the generated observer
  `committed` callback run only after `Orm::transaction` or a direct generated
  save/delete commits. Rollback discards them, and post-commit failures use a
  distinct error that says the database is already durable. These callbacks
  remain process-local; caller-owned raw SQLx transactions cannot expose their
  eventual commit decision to generated hooks.
- **Durable Transactional Outbox**: opt-in `Outbox` events commit atomically
  with domain state and use stream-scoped idempotency keys, bounded leases,
  exact claim tokens, retries and dead-letter state. PostgreSQL, MySQL,
  MariaDB and SQLite share the contract. Delivery is at least once, so the
  external consumer must also be idempotent.
- **Data Governance & Privacy Helpers**: At-rest encryption, recursive audit masking, and data-erasure primitives; legal compliance remains application-specific.
- **Scout Search Providers**: `scout-http` adds bounded Meilisearch,
  Elasticsearch and Algolia update/delete/search adapters with deterministic
  offline fallbacks. Generated projections run after commit; guaranteed crash
  recovery requires explicit outbox composition. Meilisearch has a live pinned
  container contract, while Elastic/Algolia have protocol fixtures rather than
  hosted-provider certification.
- **Typed pgvector Search**: `pgvector` re-exports SQLx-compatible `Vector` and
  the L2/cosine/inner-product helpers bind vector and distance values. A
  digest-pinned PostgreSQL + pgvector lifecycle covers typed insert/read and
  parameterized nearest-neighbor filtering/ordering; RAG orchestration and
  production index tuning remain application concerns.
- **Bounded Qdrant Search**: `qdrant` exposes validated dense-cosine collection,
  single-point upsert/delete and bounded nearest-neighbor query operations with
  deterministic fallback, authenticated protocol fixtures and a digest-pinned
  live lifecycle. Named/sparse vectors, arbitrary filters and ANN tuning remain
  explicit provider/application work.
- **Native Redis Structures**: `redis` adds a separately namespaced Hash, Set
  and Sorted Set datastore with bounded inputs/reads, TLS-required remote
  endpoints, redacted ACL credentials and a deterministic fallback. A pinned
  lifecycle covers increment, membership, ranking, exact deletion and namespace
  isolation; Lists, Streams and cluster/failover remain outside the contract.
- **Structured ORM Telemetry**: Generated/raw queries and streams emit
  `rullst.orm.query` spans without SQL, bindings or model values; managed
  transactions record bounded commit/rollback outcomes and every Rullst-owned
  pool reports checkout timing. The application's tracing/OpenTelemetry
  subscriber controls export, sampling and retention.
- **Reproducible Comparative Benchmark**: A lockfile-pinned Criterion harness
  gives Rullst, Diesel and SeaORM one typed SQLite connection, an equivalent
  indexed schema/seed/policy and the same five operations. It records evidence
  for that exact runner; the initial smoke did not support the historical
  “negligible overhead versus Diesel” claim.
- **Database-First Introspection**: The official framework CLI (`cargo rullst generate:models`) connects to legacy databases and generates your `#[derive(Orm)]` Rust structs automatically.
- **Declarative Migration Preview**: `make:migration:auto` offers a bounded
  SQLite AST/schema diff; review generated SQL before applying it.
- **Atomic Cascading Soft Deletes**: Mark generated has-one/has-many
  relationships for cascade; implicit deletes open a transaction when needed,
  while explicit or task-scoped transactions are reused without nesting.
- **Typed Partial Updates**: `.update_partial()` changes only explicitly
  selected columns and preserves model policy/tenant checks.
- **Stable Keyset Chunking**: `.chunk_by_id()` traverses ascending generated
  `i32` IDs without offset drift when already processed rows are deleted, and
  propagates callback errors. `.chunk()` remains available for offset-based
  compatibility.
- **Transaction-Aware Redis Query Cache**: `.remember(seconds)` uses a
  versioned SHA-256 key bound to the application namespace, active tenant,
  generated SQL and typed bindings. Generated reads bypass cache inside every
  ORM transaction so Redis cannot replace the transaction's database view.
  Generated model saves/deletes invalidate keys for the active tenant and table
  only after commit, using a bounded non-blocking scan; cluster/failover
  evidence remains outside the current contract.
- **Model Policies (Authorization)**: Laravel-style fine-grained access control securely tied to your structs via `#[orm(policy = "MyPolicy")]`.
- **Development lazy-loading diagnostics**: An opt-in development policy can fail loudly when a guarded lazy load would hide an N+1 query.
- **Explicit Capability Boundaries**: Unsupported replication paths fail closed instead of reporting simulated success.
- **Optional Polyglot Persistence**: Feature-gated MongoDB document CRUD,
  DuckDB OLAP queries, Turso/libSQL edge SQL, and SurrealDB document/read-only
  graph operations live behind explicit capability APIs instead of pretending
  to be one universal Active Record interface. See the
  [Polyglot guide](../docs/src/polyglot-persistence.md).
- **Bounded Turso Primary Profile**: `#[derive(Orm)]` with
  `#[orm(backend = "turso")]` exposes typed CRUD/query methods through
  `TursoOrm`, reversible checked migrations, and a persistent offline or remote
  libSQL store. The CLI currently guarantees this primary path for the
  blank/API starter; SQLx-specific relations, hooks, schema auto-diff and the
  other blueprints are not claimed as parity.

---

## 🛠️ Quick Start

### Installation

Add the library to your `Cargo.toml`:

After that RC is published, install its exact train with:

```bash
cargo add rullst-orm@12.0.0-rc.1
cargo add tokio -F full
```

### Zero-to-Hero Example

```rust
use rullst_orm::{Orm, FromRow};

// 1. Just add the Orm macro to your struct!
#[derive(Clone, FromRow, Orm)]
pub struct User {
    pub id: i32, // ID = 0 means it hasn't been saved yet
    pub name: String,
    pub email: String,
    #[orm(hidden)] // Excluded from the ORM's generated to_json() projection
    pub password: String,
}

#[tokio::main]
async fn main() -> Result<(), rullst_orm::Error> {
    // 2. Initialize the connection pool (SQLite, Postgres, MySQL/MariaDB)
    Orm::init("sqlite::memory:").await?;

    // 3. Create a new user through the generated Active Record API
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

    println!("Found {} user(s)", active_users.len());

    Ok(())
}
```

### Optional Redis query cache

Enable the `redis` feature and give each application sharing a Redis database a
stable namespace:

```rust
use rullst_orm::Orm;

Orm::init_redis_with_namespace(
    "redis://127.0.0.1:6379",
    "academy-production",
).await?;

let users = User::query().where_like("email", "%@example.com")
    .remember(30)
    .get()
    .await?;
```

An explicitly remembered query outside a transaction requires Redis
initialization. Connection/command failures and corrupt cache entries fall back
to the database, while missing configuration fails closed. Explicit and
task-scoped transactions always bypass the cache. Generated model saves and
deletes invalidate that table's generated cache keys after commit. Raw SQL,
bulk builders and writes outside generated model methods cannot be inferred, so
keep a defensive TTL and do not cache authorization or other reads whose
freshness requires a stronger distributed consistency contract.

Register process-local effects against the managed commit boundary:

```rust
use rullst_orm::{Error, Orm, after_commit};

Orm::transaction(|_| Box::pin(async move {
    // Executor-aware ORM operations participate in this transaction.
    after_commit(|| async {
        // Publish a best-effort projection after commit.
        Ok::<(), Error>(())
    }).await?;
    Ok::<(), Error>(())
})).await?;
```

For delivery that must survive a process crash, use the explicit outbox in the
same managed transaction and dispatch it from a retrying worker:

```rust
use rullst_orm::{Error, Orm, Outbox};
use serde_json::json;

Orm::transaction(|_| Box::pin(async move {
    // Persist domain state through the task-scoped transaction here.
    Outbox::enqueue(
        "tenant-42",
        "invoice:123:issued:v1",
        "invoice.issued",
        &json!({ "invoice_id": 123 }),
    ).await?;
    Ok::<(), Error>(())
})).await?;

if let Some(event) = Outbox::claim_next("tenant-42", "mail-worker-1", 30, 8).await? {
    // Deliver using (event.stream, event.event_key) as the consumer's
    // idempotency boundary, then acknowledge the exact lease token.
    Outbox::acknowledge(event.id, event.claim_key).await?;
}
# Ok::<(), Error>(())
```

`Outbox::install()` is only an explicit setup/test helper. Register
`OutboxMigration` through the application's normal migration runner in
production. Generated observers are not silently persisted, and an ACK lost
after the external effect can cause redelivery; see the
[transactional outbox tutorial](https://rullst.github.io/Rullst/book/tutorials/38-transactional-outbox.html).

---

## 📚 Documentation

We recently launched a brand-new **Interactive Documentation Hub**! 

👉 **[Explore the Full Documentation in the Rullst Book](https://rullst.github.io/Rullst/book/)**

---

## 🛡️ Security

Rullst ORM uses SQLx prepared-statement bindings for values accepted by its query builders. Structural identifiers are restricted to a bounded ASCII identifier grammar before interpolation. Raw SQL and application authorization remain the caller's responsibility; these controls reduce injection risk but are not an absolute safety guarantee.

## 📄 License
This project is licensed under the [MIT License](https://github.com/Rullst/Rullst/blob/main/LICENSE).
