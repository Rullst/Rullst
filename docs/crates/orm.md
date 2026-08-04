<div align="center">
  <h1>Rullst ORM 🌟</h1>
  <p><strong>A beautiful, type-safe, Active Record ORM for Rust.</strong></p>

  <p>
    <a href="https://crates.io/crates/rullst-orm"><img src="https://img.shields.io/crates/v/rullst-orm?style=flat-square&color=orange" alt="Crates.io" /></a>
    <a href="https://crates.io/crates/rullst-orm"><img src="https://img.shields.io/crates/d/rullst-orm?style=flat-square&color=orange" alt="Downloads" /></a>
    <a href="https://docs.rs/rullst-orm"><img src="https://img.shields.io/docsrs/rullst-orm?style=flat-square&color=blue" alt="Docs.rs" /></a>
    <a href="https://github.com/Rullst/rullst-orm/actions"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/rullst-orm/ci.yml?style=flat-square&label=Build" alt="Build Status" /></a>
    <img src="https://img.shields.io/badge/License-MIT-yellow.svg?style=flat-square" alt="License: MIT" />
  </p>
</div>

🚀 **[Visit the Official Website & Documentation Hub](https://rullst.github.io/Rullst/book/index.html)** 🚀

Built on top of `sqlx` and procedural macros, **Rullst ORM** brings the delightful, fluent syntax of Active Record frameworks directly to the high-performance Rust ecosystem.

<div align="center">
  <h3>🛡️ Enterprise-Grade Security</h3>
  <p>Rullst-ORM is built with a "Zero-Panic Policy" and tested against the most rigorous standards in the industry.<br>Our continuous pipeline guarantees absolute safety for production edge infrastructure:</p>

| Security Audit | Status | Description |
| :--- | :---: | :--- |
| **OSSF Scorecard** | <a href="https://securityscorecards.dev/viewer/?uri=github.com/Rullst/rullst-orm"><img src="https://img.shields.io/ossf-scorecard/github.com/Rullst/rullst-orm?style=flat-square&label=" alt="OSSF Scorecard" /></a> | Supply-chain security & best practices |
| **SLSA Level 3** | <a href="https://github.com/slsa-framework/slsa-verifier"><img src="https://img.shields.io/badge/SLSA-Level%203-8A2BE2?style=flat-square&label=" alt="SLSA Level 3" /></a> | Tamper-evident build provenance & supply-chain security |
| **Codecov** | <a href="https://codecov.io/gh/Rullst/rullst-orm"><img src="https://img.shields.io/codecov/c/github/Rullst/rullst-orm?style=flat-square&label=" alt="Codecov" /></a> | Strict code coverage enforcement |
| **Matrix DB Tests** | <a href="https://github.com/Rullst/rullst-orm/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/rullst-orm/ci.yml?style=flat-square&label=" alt="Testcontainers" /></a> | Dockerized PostgreSQL & MySQL integration tests |
| **OpenSSF** | <a href="https://www.bestpractices.dev/projects/13359"><img src="https://img.shields.io/cii/level/13359?style=flat-square&label=" alt="OpenSSF Best Practices" /></a> | Open source security standards |
| **Property Testing** | <a href="https://github.com/Rullst/rullst-orm/actions"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/rullst-orm/proptest.yml?style=flat-square&label=" alt="Proptest" /></a> | Validating complex logic against edge cases |
| **Miri UB Detection** | <a href="https://github.com/Rullst/rullst-orm/actions"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/rullst-orm/miri.yml?style=flat-square&label=" alt="Miri" /></a> | Detecting Undefined Behavior and memory leaks |
| **Kani Verifier** | <a href="https://github.com/Rullst/rullst-orm/actions"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/rullst-orm/kani.yml?style=flat-square&label=" alt="Kani" /></a> | Automated reasoning and formal verification |
| **CodeQL SAST** | <a href="https://github.com/Rullst/rullst-orm/actions"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/rullst-orm/codeql.yml?style=flat-square&label=" alt="CodeQL SAST" /></a> | Advanced semantic code analysis |
| **Cargo Deny** | <a href="https://github.com/Rullst/rullst-orm/actions"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/rullst-orm/cargo-deny.yml?style=flat-square&label=" alt="Cargo Deny" /></a> | Banning unmaintained/vulnerable crates |
| **Cargo Audit** | <a href="https://github.com/Rullst/rullst-orm/actions"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/rullst-orm/auto-audit.yml?style=flat-square&label=" alt="Auto-Audit" /></a> | Continuous scanning for crate vulnerabilities |
| **Cargo SemVer** | <a href="https://github.com/Rullst/rullst-orm/actions"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/rullst-orm/semver.yml?style=flat-square&label=" alt="cargo-semver-checks" /></a> | Strict SemVer API breakage checks |
| **Cargo Machete** | <a href="https://github.com/Rullst/rullst-orm/actions"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/rullst-orm/cargo-machete.yml?style=flat-square&label=" alt="Cargo Machete" /></a> | Detecting unused and bloated dependencies |
| **Continuous Fuzzing** | <a href="https://github.com/Rullst/rullst-orm/actions"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/rullst-orm/fuzz.yml?style=flat-square&label=" alt="Fuzzing" /></a> | Fuzzing against edge cases & panics |
| **Mutation Testing** | <a href="https://github.com/Rullst/rullst-orm/actions"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/rullst-orm/mutants.yml?style=flat-square&label=" alt="Mutants" /></a> | Mutation testing for test suite robustness |
| **Continuous Benchmarks** | <a href="https://rullst.github.io/rullst-orm/"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/rullst-orm/bench.yml?style=flat-square&label=" alt="Benchmarks CI" /></a> | Continuous performance regression testing & live dashboard |
| **Unsafe Policy** | <a href="https://github.com/Rullst/rullst-orm/actions"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/rullst-orm/unsafe-policy.yml?style=flat-square&label=" alt="Unsafe Policy" /></a> | 100% memory safe. No unsafe code blocks |
| **Panic Policy** | <a href="https://github.com/Rullst/rullst-orm/actions"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/rullst-orm/panic-policy.yml?style=flat-square&label=" alt="Panic Policy" /></a> | Graceful error handling across the framework |



</div>

## 🚀 Why Rullst ORM?

In traditional Rust database handling, you have to write raw SQL queries, manage connection pools manually, and bind variables repetitively. Rullst ORM abstracts the heavy lifting behind a single `#[derive(Orm)]` macro, generating hundreds of safe, chainable query methods at compile time.

**Key Features:**
- **Zero-Boilerplate CRUD**: Insert, update, delete, and find records instantly.
- **Fluent Query Builder**: Chain `.where_eq()`, `.limit()`, and `.order_by()` effortlessly.
- **Eager Loading**: Solve N+1 problems with robust `has_many`, `belongs_to`, and `morph_many` relations.
- **Built-in Multi-Tenancy**: Automatically scope all queries by tenant ID.
- **Automated Audit Logs**: Track `old_values` and `new_values` history natively.
- **Data Governance & Privacy**: Compile-time GDPR/LGPD compliance with at-rest encryption and log masking.
- **Scout Search**: Seamlessly sync models to full-text search engines.
- **Database-First Introspection**: The official framework CLI (`cargo rullst generate:models`) connects to legacy databases and generates your `#[derive(Orm)]` Rust structs automatically.
- **Declarative Struct-Based Migrations**: Safely auto-generate additive SQL migrations (`make:migration:auto`) directly from your Rust struct definitions.
- **Cascading Soft Deletes**: Configure relationships to automatically soft-delete dependent children records in a single transaction.
- **Type-Safe Partial Updates**: Virtual dirty checking with `.update_partial()` to intelligently modify only changed columns.
- **Model Policies (Authorization)**: Laravel-style fine-grained access control securely tied to your structs via `#[orm(policy = "MyPolicy")]`.
- **Strict Lazy Loading Prevention**: Enable a global toggle to instantly panic on N+1 queries during development.
- **Enterprise Ready**: Read/write replica splitting, query chunking, and Redis caching built-in.

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

Rullst ORM employs rigorous defenses against **SQL Injection**. All dynamic builder methods (like `.where_eq()`) automatically escape values using `sqlx` prepared statement bindings (`$1` or `?`). Raw queries (`.where_raw()`) actively force developers to provide an array of bindings directly in the function signature. Furthermore, all structural identifiers (table and column names) are validated strictly at runtime against a highly-optimized O(N) linear byte scan (zero regex overhead) to guarantee absolute SQL safety without sacrificing performance.

## 📄 License
This project is licensed under the [MIT License](LICENSE).
