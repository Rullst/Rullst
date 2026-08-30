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

🚀 **[Visit the Official Website & Documentation Hub](https://rullst.github.io/Rullst/book/index.html)** 🚀

Built on top of `sqlx` and procedural macros, **Rullst ORM** brings the delightful, fluent syntax of Active Record frameworks directly to the high-performance Rust ecosystem.

<div align="center">
  <h3>🛡️ Security Engineering</h3>
  <p>Rullst ORM uses SQLx bindings, validated identifiers, typed errors, and layered CI checks. Workflow badges are scoped test results, not a guarantee for an application or deployment.</p>

| Security Audit | Status | Description |
| :--- | :---: | :--- |
| **OSSF Scorecard** | <a href="https://securityscorecards.dev/viewer/?uri=github.com/Rullst/Rullst"><img src="https://img.shields.io/ossf-scorecard/github.com/Rullst/Rullst?style=flat-square&label=" alt="OSSF Scorecard" /></a> | Supply-chain security & best practices |
| **Release Provenance** | <a href="https://github.com/Rullst/Rullst/actions/workflows/release.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/release.yml?style=flat-square&label=" alt="Release provenance" /></a> | Provenance attestations for release artifacts; no SLSA level is claimed here |
| **Codecov** | <a href="https://codecov.io/gh/Rullst/Rullst"><img src="https://img.shields.io/codecov/c/github/Rullst/Rullst?style=flat-square&label=" alt="Codecov" /></a> | Strict code coverage enforcement |
| **Matrix DB Tests** | <a href="https://github.com/Rullst/Rullst/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/ci.yml?style=flat-square&label=" alt="Testcontainers" /></a> | Live PostgreSQL, MySQL, MariaDB, MongoDB, SurrealDB and libSQL contracts, plus in-process DuckDB tests |
| **OpenSSF** | <a href="https://www.bestpractices.dev/projects/13359"><img src="https://img.shields.io/cii/level/13359?style=flat-square&label=" alt="OpenSSF Best Practices" /></a> | Open source security standards |
| **Property Testing** | <a href="https://github.com/Rullst/Rullst/actions/workflows/proptest.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/proptest.yml?style=flat-square&label=" alt="Proptest" /></a> | Validating complex logic against edge cases |
| **Miri UB Detection** | <a href="https://github.com/Rullst/Rullst/actions/workflows/miri.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/miri.yml?style=flat-square&label=" alt="Miri" /></a> | Detecting Undefined Behavior and memory leaks |
| **Kani Verifier** | <a href="https://github.com/Rullst/Rullst/actions/workflows/kani.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/kani.yml?style=flat-square&label=" alt="Kani" /></a> | Automated reasoning and formal verification |
| **CodeQL SAST** | <a href="https://github.com/Rullst/Rullst/actions/workflows/codeql.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/codeql.yml?style=flat-square&label=" alt="CodeQL SAST" /></a> | Advanced semantic code analysis |
| **Cargo Deny** | <a href="https://github.com/Rullst/Rullst/actions/workflows/cargo-deny.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/cargo-deny.yml?style=flat-square&label=" alt="Cargo Deny" /></a> | Banning unmaintained/vulnerable crates |
| **Cargo Audit** | <a href="https://github.com/Rullst/Rullst/actions/workflows/audit.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/audit.yml?style=flat-square&label=" alt="Auto-Audit" /></a> | Continuous scanning for crate vulnerabilities |
| **Cargo SemVer** | <a href="https://github.com/Rullst/Rullst/actions/workflows/semver.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/semver.yml?style=flat-square&label=" alt="cargo-semver-checks" /></a> | Strict SemVer API breakage checks |
| **Cargo Machete** | <a href="https://github.com/Rullst/Rullst/actions/workflows/machete.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/machete.yml?style=flat-square&label=" alt="Cargo Machete" /></a> | Detecting unused and bloated dependencies |
| **Continuous Fuzzing** | <a href="https://github.com/Rullst/Rullst/actions/workflows/fuzzing.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/fuzzing.yml?style=flat-square&label=" alt="Fuzzing" /></a> | Fuzzing against edge cases & panics |
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
- **Audit Log Foundation**: Opt-in auditable models can record bounded
  `old_values`/`new_values` diffs. Generated instance saves/deletes and their
  audit row now commit or roll back one savepoint together and audit failures
  reject the mutation. Bulk builders do not synthesize per-row history; actor
  identity, revision restore, and durable post-commit effects remain
  application/roadmap work.
- **Data Governance & Privacy Helpers**: At-rest encryption, recursive audit masking, and data-erasure primitives; legal compliance remains application-specific.
- **Scout Extension Point**: Connect an application-owned search engine to the
  generated search/save/delete hooks; live Meili/Algolia/Elastic adapters are
  not bundled yet.
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

```bash
cargo add rullst-orm
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

    // 3. Create a new user magically
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

    println!("Found {} user(s)", active_users.len());

    Ok(())
}
```

---

## 📚 Documentation

We recently launched a brand-new **Interactive Documentation Hub**! 

👉 **[Explore the Full Documentation in the Rullst Book](https://rullst.github.io/Rullst/book/index.html)**

---

## 🛡️ Security

Rullst ORM uses SQLx prepared-statement bindings for values accepted by its query builders. Structural identifiers are restricted to a bounded ASCII identifier grammar before interpolation. Raw SQL and application authorization remain the caller's responsibility; these controls reduce injection risk but are not an absolute safety guarantee.

## 📄 License
This project is licensed under the [MIT License](https://github.com/Rullst/Rullst/blob/main/LICENSE).
