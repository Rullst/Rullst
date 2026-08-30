# Rullst ORM Roadmap

> **Status policy (audited 2026-08-29):** this roadmap preserves design history
> and ambition. `[x]` means implemented only inside the bounded contract linked
> from the v12 audit, `[~]` means a real but incomplete foundation, and `[ ]`
> means absent. See the item-by-item
> [v12 historical audit](../docs/src/v12.md#inventário-item-a-item--rullst-orm-histórico)
> and the [capability ledger](../docs/src/capability-ledger.md).

Rullst ORM aims for an inspectable, AI-friendly Active Record experience while
retaining Rust typing, explicit escape hatches and parameterized values.

## Implemented foundations

- Active Record models via `#[derive(Orm)]` and repository abstractions.
- Fluent asynchronous query builder and generated typed column helpers.
- Pagination, pluck, eager loading, lifecycle hooks, scopes and soft deletes.
- Has-one, has-many, belongs-to and many-to-many relationships.
- SQLx schema/migration foundations for PostgreSQL, MySQL/MariaDB and SQLite.
- A separate typed Turso-primary profile and bounded polyglot capability APIs.

## Phase 2: advanced features

- [x] **Database Transactions**: `Orm::transaction` and explicit transaction handles cover generated/executor-aware operations with commit/rollback tests; direct caller-owned SQL must use the supplied transaction or executor macro.
- [x] **ORM Collections**: `map`, `pluck`, `key_by` and other collection helpers.
- [x] **Compile-Time Column/Value Safety (bounded)**: Generated primitive-field
  filters bind the real persisted type and typed column enums reject unknown
  names at compile time. Dynamic string-column and raw SQL APIs remain explicit
  runtime-checked/caller-owned escape hatches.
- [~] **Polymorphic Relationships**: `morphMany` and `morphOne` exist; inverse `morphTo` does not.
- [x] **Factories and Seeders**: Fluent factory `make`/`create` plus the async `Seeder` contract; applications provide deterministic fake-data generators.

## Phase 3: richer Active Record

- [x] **Many-to-Many Relationships**: Pivot-table loading and bounded eager loading.
- [x] **Pagination with Metadata**: Positive page sizes return total/current/last-page metadata.
- [x] **JSON Column Casting**: `#[orm(json)]`/`Json<T>` supports typed serialization and row mapping.
- [x] **Constrained Eager Loading**: Generated relationship filters accept typed builder closures.
- [x] **Migrations CLI**: Generate, run, status and rollback commands with per-success tracking; Turso has a distinct reversible/checksummed runner.
- [x] **Observers & Lifecycle Events**: Per-model process-wide async observer registry for create/update/delete events.
- [x] **Subqueries & Advanced Joins**: Subquery/CTE and validated join builders with parameterized values.
- [x] **Database Seeding**: `db:seed` runs application-registered seeders.
- [x] **Query Logging & Debugging**: Opt-in SQL logging redacts bound values.
- [~] **Model Serialization (Hiding Fields)**: `#[orm(hidden)]` is honored by generated `to_json`; a separately derived Serde serializer is not rewritten.

## Phase 4: scale and observability

- [~] **Edge Native & Read Replicas**: Configured SQLx reads rotate over replicas and writes use primary; transparent Turso synchronization and latency guarantees do not exist.
- [x] **Query Chunking & Cursors (bounded)**: `.chunk(...)` preserves offset
  compatibility, while fallible `.chunk_by_id(...)` and its transaction-aware
  counterpart traverse the generated `i32` primary key with stable keyset
  pagination. This is not a database-server cursor or a cross-shard snapshot.
- [x] **Async Streams**: Query builders expose bounded `futures::Stream` row iteration.
- [~] **Integrated Caching Layer**: `.remember(seconds)` has a Redis implementation, but no live Redis conformance/invalidation contract is currently claimed.
- [~] **Asynchronous Reactive Event Hooks**: Async observers and optional Redis publish exist; a durable, strictly post-commit outbox does not.
- [~] **Security & Performance Static Audit**: Current gates and targeted hardening are tracked; an old third-party audit cannot become a timeless “all findings resolved” guarantee.
- [ ] **Continuous Performance Comparison with Diesel and SeaORM**: Rullst Criterion benchmarks exist, but the historical cross-ORM comparison/proof does not.
- [~] **Native OpenTelemetry**: Query spans use `tracing`; transaction/checkout coverage and an OpenTelemetry export contract are incomplete.
- [~] **Raw SQL Mapping Fallback**: `Orm::raw(...).bind(...).map_to::<T>()` maps rows, but raw SQL remains runtime-checked and caller-owned.

## Phase 5: architecture choice

Rullst intentionally favors an owned, lifetime-light query API over a universal
“zero-copy” promise. Strict PostgreSQL/MySQL/SQLite features select a concrete
SQLx backend; they improve type coherence but do not make dynamic SQL
compile-time schema verified.

## Phase 6: ecosystem integrations

- [x] **Native Multi-tenancy**: `tenant_column` queries and mutations fail closed without `with_tenant(...)`; cross-tenant instance mutations are rejected and global access requires explicit `unscoped()`.
- [~] **Declarative Struct-Based Migrations**: SQLite AST/schema diff scaffolding exists, but it is not a type-complete or universally safe synchronizer.
- [~] **Declarative Destructive Migrations**: SQLite diff emits destructive suggestions commented out; full synchronization and an `--allow-destructive` execution contract do not exist.
- [~] **Strict Lazy Loading Prevention**: Explicit generated relationship access fails while prevention is enabled; it cannot prove absence of every application-level N+1 pattern.
- [~] **Type-Safe Partial Updates**: A typed explicit builder emits only selected fields and preserves policy/tenant checks; it is not automatic dirty tracking or a zero-overhead proof.
- [~] **Compliance & Data Governance Foundations**: `PersonalData`, redacted `SecretString`, and AES-GCM encrypted model fields are separate bounded primitives, not automatic GDPR/LGPD compliance.
- [~] **Audit Trails**: Diff history exists for auditable model writes; actor identity, durable rollback, fail-closed writes and strict transaction coupling remain incomplete.
- [~] **Full-Text Search (Scout)**: A `SearchEngine` extension point and save/delete hooks exist; Meilisearch, Algolia and Elasticsearch adapters are not included.
- [x] **Sandbox Testing**: `#[rullst_orm::test]` scopes executor-aware operations to a transaction and rolls it back after the async test.
- [~] **Model Policies**: Generated create/update/delete/restore checks exist, including partial updates; read authorization and host identity/ownership loading remain application work.
- [~] **ORM Admin Panel**: A static dashboard shell exists; authenticated data-management CRUD belongs to Nexus.
- [x] **API Resources & Transformers**: Explicit resource and collection transforms generate bounded JSON projections.
- [~] **Distributed Graph Traversal**: Manual recursive CTE helpers exist; automatic relationship-graph traversal and distribution do not.
- [x] **Polyglot Persistence (bounded v12)**: Optional MongoDB document CRUD, DuckDB parameterized/bounded OLAP, Turso/libSQL SQL/transactions/migrations and SurrealDB HTTP document/read-only GQL adapters preserve separate semantics.
- [~] **Advanced Vector & Key-Value Stores**: Optional Redis cache/hash helpers exist; Qdrant and a general native Redis datastore contract do not.
- [x] **Schema Visualizer**: `cargo rullst generate:diagram` emits Mermaid from statically inspected models.
- [~] **Cascading Soft Deletes**: Marked `has_one`/`has_many` relations cascade;
  implicit deletes now create an atomic transaction when necessary and reuse
  explicit/task-scoped transactions. Recursive descendant traversal and cycle
  handling remain outside the bounded contract.
- [~] **Native Enum Mapping**: Rust enums map to validated strings and schema helpers emit portable `CHECK`; native PostgreSQL/MySQL enum DDL is not generated.

## Phase 7: future and infrastructure

- [x] **Database-First Introspection**: `cargo rullst generate:models` supports bounded PostgreSQL, MySQL/MariaDB and SQLite schema-to-model generation.
- [~] **Vector Query Helpers (`pgvector` syntax)**: Validated finite-vector helpers emit pgvector operators; extension lifecycle, live PostgreSQL conformance and full RAG orchestration remain outside the contract.
- [ ] **AI-Powered Auto Migrations**: Any future implementation must be opt-in, previewed and reviewed; autonomous production DDL is not recommended.
- [ ] **Wasm & Edge Computing**: No supported browser/Cloudflare/Vercel ORM runtime exists.
- [~] **ORM Sail**: `sail:install` writes a Compose starting point for Postgres, Redis, Meilisearch and pgAdmin; it does not start services or scaffold the application container.
- [ ] **Post-Quantum Field Encryption**: No production `#[orm(encrypt_pq)]` implementation exists; custom cryptography is not recommended.
- [ ] **Automatic Distributed Graph Traversal**: Manual CTE and bounded SurrealDB read-only GQL are separate foundations, not this capability.
- [ ] **Qdrant / General Redis Datastore**: Not implemented beyond the bounded Redis helpers above.
- [x] **Edge Databases (bounded v12)**: Remote-only official libSQL support provides parameterized Turso SQL, transactions, bounded materialization, checksummed migrations and real-SQL offline fallback; transparent replicas and SQLx Active Record parity are not claimed.
