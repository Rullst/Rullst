# Rullst ORM Roadmap

**The Emotional Productivity ORM**
To beat competitors like Diesel (focused on complex mathematical safety) and SeaORM (focused on traditional Active Record inheritance), Rullst-ORM positions itself as the "Emotional Productivity ORM":
- **Zero black boxes**: All SQL generated under the hood should be easily inspectable via structured development logs.
- **AI-Friendly**: Clean structures that facilitate LLM reading, generating routines without syntax hallucinations.

Our goal is to bring the best of the *Every ORM* experience to the Rust ecosystem.
Here we track the key features that differentiate Rullst ORM from other query builders and our implementation status.

## ✅ Implemented
- **Active Record/Models**: Structs directly connected to the database (`#[derive(Orm)]`).
- **Fluent Query Builder**: Method chaining (`.where_eq()`, `.order_by()`, etc).
- **Asynchronous Execution**: Powered by Tokio + SQLx.
- **Basic Magic Methods**: `.where_name("...").where_email("...")`.
- **Pagination**: `.paginate()` method to return paginated results and meta-information easily.
- **Auto Timestamps**: Native control of `created_at` and `updated_at` in `save/update/insert` methods.
- **Helper Methods**: `.first_or_fail()`, `.find_or_fail()`.
- **Pluck**: Fetching a single column.
- **Eager Loading**: N+1 problem prevention using `.with("comments")`.
- **Mutators and Accessors**: Handling data transformation via lifecycle hooks.
- **Events and Observers**: Handling hooks like `before_save`, `after_fetch`, etc.
- **Local and Global Scopes**: Reusable query constraints.
- **Soft Deletes**: Logical deletion hiding the record (`deleted_at` column).
- **Relationships**: `HasOne`, `HasMany`, `BelongsTo`, `BelongsToMany`.
- **Migrations**: Fluent schema builder API for creating tables.

## 🎉 Phase 1 Completed!
All core features of Laravel Orm have been successfully ported to Rust.

## 🚀 Phase 2: Advanced Features & Rust Superpowers
- [x] **Database Transactions**: Wrapping queries in transactional blocks (`Orm::transaction`).
- [x] **Orm Collections**: Custom collection struct with high-level methods (`map`, `pluck`, `key_by`).
- [x] **Compile-Time Safety**: Using Rust's strict typing and macros to check SQL columns at compile-time.
- [x] **Polymorphic Relationships**: `morphTo`, `morphMany`, `morphOne`.
- [x] **Factories and Seeders**: Fluent API for generating fake testing data.

## 👑 Phase 3: The Rust Masterpiece
- [x] **Many-to-Many Relationships**: Implement pivot table support (`belongsToMany`).
- [x] **Pagination with Metadata**: `.paginate(15)` returning total, current page, and data.
- [x] **JSON Column Casting**: `#[orm(json)]` macro parameter to auto-deserialize `serde_json` structs.
- [x] **Constrained Eager Loading**: Passing closures to relationships like `.with_posts_constrained(|q| q...)`.
- [x] **Rust Artisan (Migrations CLI)**: Command-line tool to generate, run, and rollback database migrations.
- [x] **Observers & Lifecycle Events**: Global observer pattern to listen to model events (`creating`, `saved`, `deleted`) externally.
- [x] **Subqueries & Advanced Joins**: Allowing closures for complex SQL joins and subqueries.
- [x] **Artisan Seeding (db:seed)**: Populate tables via Artisan CLI using Seeders and Factories.
- [x] **Query Logging & Debugging**: Inspect the executed SQL directly in terminal for optimization.
- [x] **Model Serialization (Hiding Fields)**: Attribute `#[orm(hidden)]` to automatically skip sensitive columns during JSON serialization.

## 🏭 Phase 4: Enterprise Scale (v1.0.0)
- [x] **Edge Native & Read Replicas**: Automatic connection splitting. The ORM intelligently identifies read operations and routes `SELECT` to the local edge replica (e.g., Turso, 1ms latency), while transparently sending `INSERT/UPDATE` via transaction to the global primary database.
- [x] **Query Chunking & Cursors**: Methods like `.chunk(1000, |batch| ...)` to process millions of records safely without high memory usage.
- [x] **Async Streams**: Support for `futures::Stream` (`impl Stream<Item = Model>`) to asynchronously iterate over millions of records with minimal memory footprint.
- [x] **Integrated Caching Layer**: Add `.remember(seconds)` using an optional Redis feature flag to automatically cache repetitive queries.
- [x] **Asynchronous Reactive Event Hooks (Data Middlewares)**: Optional pub/sub event broadcasting and lifecycle hooks (like `after_commit`) based on Tokio's async/await. External events (webhooks, clearing Redis cache) trigger strictly only if the database transaction is confirmed with absolute success.
- [x] **Security & Performance Static Audit**: All critical and medium-priority findings from the Jules/Antigravity architecture audit resolved in v1.1.13 (QueryBuilder binding fix, error propagation, clippy compliance).
- [x] **Continuous Performance Benchmarks**: Automated CI regression tests using Criterion to prove that the ergonomic heap-allocations (abandoning Zero-Copy) have negligible impact compared to Diesel and SeaORM.
- [x] **Native OpenTelemetry (Tracing)**: Deep integration with the `tracing` crate to automatically emit spans for queries, transactions, and connection checkout for observability.
- [x] **Type-Safe Raw SQL Fallback**: A seamless method like `Orm::raw("SELECT * FROM users").map_to::<User>()` for complex edge cases without losing ORM mapping capabilities.

## 🔮 Phase 5: Version 3.0.0 Architecture (Completed)

With the release of `v3.0.0`, we successfully rebranded from Eloquent to Rullst and solidified our architectural direction. We made a conscious design decision to **abandon the "Zero-Copy" (`std::borrow::Cow`) architecture** that was previously planned for the query builder.

**Why abandon Zero-Copy?** 
Rullst ORM is built on the philosophy of extreme developer productivity (Laravel-like ease of use). Introducing lifetimes (`<'a>`) into the public API would force developers to fight the Rust borrow checker during standard database operations, entirely defeating the purpose of the library. We prioritize ergonomics, and the negligible overhead of heap `String` allocation is a tradeoff we gladly accept for a clean, lifetime-free API.

Instead, we achieved **Compile-Time Safety** without lifetimes through our Strict Feature Flags:

### 🛡️ Strict SQL Typing (Delivered via Feature Flags)
We introduced the `strict-postgres`, `strict-mysql`, and `strict-sqlite` feature flags. 
- When enabled, the ORM bypasses the dynamic `sqlx::AnyPool` and natively binds to the specific database driver, enabling strict compile-time verification without polluting the user's code with lifetimes.
- The default behavior remains dynamically typed, ensuring maximum flexibility for rapid prototyping.

## 🌍 Phase 6: The Ultimate Ecosystem (SaaS & Open Source Mastery)

Our goal is to provide tools that normally cost thousands of dollars, completely free and open-source, ensuring `rullst-orm` stands unrivaled in the Rust ecosystem.

- [x] **Native Core Multi-tenancy (Global Scopes)**: Automatic Global Scopes isolate tenant data in SaaS. With `#[orm(global_scope = "tenant")]`, the ORM injects `WHERE tenant_id = X` and prevents leaks; requiring global access demands explicit and noisy calls like `.unscoped()`.
- [x] **Declarative Struct-Based Migrations (Safe Additive)**: Automatic synchronization reading Rust structs to generate corresponding SQL migrations (tables and columns) via the `make:migration:auto` command, guaranteeing data safety.
- [ ] **Declarative Migrations (Destructive Operations)**: Implement full resource synchronization (dropping columns and tables) protected by "Safe by Default" patterns (e.g., `--allow-destructive` flag or commented-out SQL generation) to prevent accidental data loss.
- [x] **Strict Lazy Loading Prevention**: A Laravel-inspired feature (`Orm::prevent_lazy_loading(true)`) that throws a loud runtime error during development/testing if a relationship is accessed without being eager-loaded, completely preventing N+1 issues without polluting the codebase with strict typestates.
- [x] **Type-Safe Partial Updates (Virtual Dirty Checking)**: A macro-driven mechanism that tracks changed properties in memory, generating an `UPDATE` only for modified columns without overhead, using typing to honor database constraints (e.g., not null).
- [x] **Automated Compliance & Data Governance (GDPR/LGPD)**: `#[derive(PersonalData)]` macro for out-of-the-box privacy reports, and `SecretString` for transparent AES-256-GCM encryption at rest, preventing accidental data leakage.
- [x] **Audit Trails (Revision History)**: A `#[orm(auditable)]` macro that automatically tracks "who changed what" in a separate history table for compliance and rollbacks.
- [x] **Built-in Full-Text Search (Scout)**: `.search("query")` method that automatically syncs your models with Meilisearch, Algolia, or Elasticsearch upon saving.
- [x] **Sandbox Testing (RefreshDatabase)**: A testing macro/utility that wraps each automated test in a database transaction and automatically rolls it back at the end, ensuring isolated and fast test executions just like Laravel.
- [x] **Model Policies (Authorization)**: A declarative way (via attributes like `#[orm(policy = "PostPolicy")]`) to define fine-grained access control rules directly tied to the models.
- [x] **Rullst ORM Admin Panel**: A drop-in function that generates a beautiful web dashboard to manage your data without writing frontend code.
- [x] **API Resources & Transformers**: A declarative way to transform Rullst Models and eager-loaded relationships into clean JSON API responses, handling hidden fields, date formatting, and nested relations effortlessly.
- [x] **Schema Visualizer (Mermaid ER Diagrams)**: A CLI command (`cargo rullst generate:diagram`) that reads models and outputs a Mermaid markdown file showing database architecture and relationships.
- [x] **Cascading Soft Deletes**: Automatic recursive soft deletion. If a parent is soft-deleted, automatically apply soft deletes to dependent children (`ON DELETE CASCADE` equivalent for logical deletes).
- [x] **Native Enum Mapping**: Clean, type-safe mapping of Rust `enum` to database `ENUM` types (PostgreSQL/MySQL) or secure string constraints (SQLite).

## 🧠 Phase 7: The Future (AI, Quantum & Infrastructure)

Pushing the boundaries of what an ORM can do in the modern era of computing.

- [x] **Database-First Introspection**: An Artisan CLI tool (`cargo rullst generate:models`) to connect to legacy databases and automatically generate Rust structs mapped to existing tables.
- [x] **Native Vector DB & RAG Support (`pgvector`)**: Methods like `.where_similar("embedding", vector)` and `.order_by_similarity()` to natively support AI applications and Retrieval-Augmented Generation directly in standard SQL databases.
- [ ] **AI-Powered Auto Migrations**: An opt-in tool that analyzes your Rust structs and uses a local or remote LLM to automatically generate the perfect SQL migration diffs, eliminating manual SQL typing.
- [ ] **Wasm & Edge Computing**: Running the ORM directly on Cloudflare Workers or Vercel Edge with Serverless DB drivers (PlanetScale, Neon).
- [x] **Orm Sail (Instant Docker)**: A CLI command that instantly spins up a `docker-compose` environment with Postgres, Redis, Meilisearch, and your Rust app pre-configured. Zero infra setup.
- [ ] **Post-Quantum Cryptography**: A `#[orm(encrypt_pq)]` macro to encrypt sensitive columns (like medical records, passwords) at rest using post-quantum algorithms (e.g., CRYSTALS-Kyber) to future-proof against quantum computer attacks.
- [ ] **Distributed Graph Traversal**: Transforming standard SQL tables into Graph-like queries for deep recursive relationships (e.g., `friends.of.friends`) using advanced CTEs automatically generated by the ORM.
- [ ] **Polyglot Persistence (NoSQL & Analytics)**: Extend the ORM beyond relational databases with driver support for MongoDB (document), DuckDB (OLAP/analytics), and SurrealDB (multi-model/graph).
- [ ] **Advanced Vector & Key-Value Stores**: First-class integration for specialized datastores like Qdrant (advanced vector search) and native Redis data structures (beyond just caching).
- [ ] **Edge Databases**: Full native support for Turso (libsql) to power distributed computing at the edge.
