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
| **Matrix DB Tests** | <a href="https://github.com/Rullst/Rullst/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/ci.yml?style=flat-square&label=" alt="Testcontainers" /></a> | Dockerized PostgreSQL & MySQL integration tests |
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
- **Opt-in audit logs:** `#[orm(auditable)]` records model changes after the
  audit table is created. Generated instance saves/deletes and their audit entry
  share a savepoint and fail together; bulk builders do not synthesize per-row
  history. Sensitive names are recursively masked, and explicitly encrypted
  fields are decrypted only in memory before the masked diff is built. Actor
  identity and revision restore remain application responsibilities.
- **Field privacy:** `#[orm(encrypted)]` transparently encrypts supported
  `String` fields with a versioned AES-256-GCM envelope. Randomized ciphertext
  cannot be filtered or sorted; use a separate keyed blind index where needed.
- **Scout hooks:** `#[orm(searchable)]` calls a configured `SearchEngine` after
  generated writes/deletes. Delivery guarantees depend on the adapter.
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

👉 **[Explore the Full Documentation in the Rullst Book](https://rullst.github.io/Rullst/book/index.html)**

---

## 🛡️ Security

Rullst ORM uses SQLx prepared-statement bindings for values accepted by its query builders. Structural identifiers are restricted to a bounded ASCII identifier grammar before interpolation. Raw SQL and application authorization remain the caller's responsibility; these controls reduce injection risk but are not an absolute safety guarantee.

## 📄 License
This project is licensed under the [MIT License](LICENSE).
