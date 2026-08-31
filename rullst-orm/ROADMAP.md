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
- [x] **Polymorphic Relationships (bounded)**: `morph_many`, `morph_one`, and
  explicit typed `morph_to` targets support lazy and batched eager loading. The
  discriminator is the Rust target name; universal runtime type registries and
  cross-store polymorphism are outside this contract.
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
- [~] **Integrated Caching Layer**: `.remember(seconds)` uses versioned
  application/tenant-aware SHA-256 keys and bypasses explicit/task-scoped
  transactions. Missing configuration fails closed; transport/corrupt-entry
  failures use the database. Generated model save/delete invalidation is
  tenant/table-scoped, bounded and post-commit. A pinned single-node Redis gate covers
  hit, TTL, corrupt-entry recovery, invalidation and rollback preservation;
  raw/bulk writes and cluster/failover evidence remain outside the contract.
- [~] **Asynchronous Reactive Event Hooks**: `after_commit`, generated
  `committed` observers, Redis invalidation/pub-sub and Scout projection use the
  managed transaction commit boundary. Rollback drops effects and a distinct
  error reports failures after persistence. The opt-in relational `Outbox`
  now supplies atomic idempotent enqueue, lease/token claims, bounded retry and
  dead-letter across PostgreSQL, MySQL/MariaDB and SQLite. Generated hooks are
  still process-local and are not automatically serialized into application
  events; external consumers must be idempotent and caller-owned raw SQLx
  transactions cannot expose their later decision.
- [~] **Security & Performance Static Audit**: Current gates and targeted hardening are tracked; an old third-party audit cannot become a timeless “all findings resolved” guarantee.
- [x] **Continuous Performance Comparison with Diesel and SeaORM (bounded)**:
  the pinned Criterion harness runs equivalent typed-SQLite find, filtered
  read, count, list-ten and insert/delete shapes through one connection for
  Rullst, Diesel and SeaORM. CI publishes per-commit history. It measures that
  exact runner/schema/configuration and deliberately makes no universal or
  “negligible overhead” claim.
- [x] **Native OpenTelemetry (bounded)**: Generated/raw query and stream spans, managed transaction outcomes and SQLx pool-acquire timing use secret-free structured `tracing` metadata. Core's opt-in OpenTelemetry layer exports them; the host still owns subscriber setup, sampling, collector policy and direct SQLx/application logging.
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
- [~] **Audit Trails**: Diff history is fail-closed and savepoint-coupled to generated auditable instance saves/deletes; bulk per-row history, actor identity, revision restore, and durable post-commit external effects remain incomplete.
- [~] **Full-Text Search (Scout)**: The `scout-http` feature supplies bounded
  Meilisearch, Algolia and Elasticsearch update/delete/search adapters with
  deterministic offline fallbacks. A digest-pinned Meilisearch lifecycle is
  live; Algolia/Elasticsearch use protocol fixtures. Generated hooks remain
  process-local unless the application explicitly composes the durable outbox;
  hosted-provider, failover and ranking evidence remain external.
- [x] **Sandbox Testing**: `#[rullst_orm::test]` scopes executor-aware operations to a transaction and rolls it back after the async test.
- [~] **Model Policies**: Generated create/update/delete/restore checks exist, including partial updates; read authorization and host identity/ownership loading remain application work.
- [~] **ORM Admin Panel**: A static dashboard shell exists; authenticated data-management CRUD belongs to Nexus.
- [x] **API Resources & Transformers**: Explicit resource and collection transforms generate bounded JSON projections.
- [~] **Distributed Graph Traversal**: Manual recursive CTE helpers exist; automatic relationship-graph traversal and distribution do not.
- [x] **Polyglot Persistence (bounded v12)**: Optional MongoDB document CRUD, DuckDB parameterized/bounded OLAP, Turso/libSQL SQL/transactions/migrations and SurrealDB HTTP document/read-only GQL adapters preserve separate semantics.
- [x] **Advanced Vector & Key-Value Stores (bounded)**: Qdrant exposes validated dense-cosine collection/upsert/delete/query operations; Redis exposes namespaced Hash, Set and Sorted Set operations. Both select deterministic empty/`mock_*` fallbacks and pass digest-pinned live lifecycles. Named/sparse Qdrant vectors, arbitrary filters, Redis Lists/Streams and distributed topology remain outside this contract.
- [x] **Schema Visualizer**: `cargo rullst generate:diagram` emits Mermaid from statically inspected models.
- [~] **Cascading Soft Deletes**: Marked `has_one`/`has_many` relations cascade;
  implicit deletes now create an atomic transaction when necessary and reuse
  explicit/task-scoped transactions. Recursive descendant traversal and cycle
  handling remain outside the bounded contract.
- [~] **Native Enum Mapping**: Rust enums map to validated strings and schema helpers emit portable `CHECK`; native PostgreSQL/MySQL enum DDL is not generated.

## Phase 7: future and infrastructure

- [x] **Database-First Introspection**: `cargo rullst generate:models` supports bounded PostgreSQL, MySQL/MariaDB and SQLite schema-to-model generation.
- [x] **Vector Query Helpers (bounded pgvector contract)**: `pgvector` plus
  `strict-postgres` exposes the typed SQLx value and parameterized L2, cosine
  and inner-product helpers. A digest-pinned live lifecycle installs the
  extension and proves typed inserts/queries. RAG orchestration, application
  authorization and production ANN index tuning remain separate concerns.
- [ ] **AI-Powered Auto Migrations**: Any future implementation must be opt-in, previewed and reviewed; autonomous production DDL is not recommended.
- [ ] **Wasm & Edge Computing**: No supported browser/Cloudflare/Vercel ORM runtime exists.
- [~] **ORM Sail**: `sail:install` writes a Compose starting point for Postgres, Redis, Meilisearch and pgAdmin; it does not start services or scaffold the application container.
- [ ] **Post-Quantum Field Encryption**: No production `#[orm(encrypt_pq)]` implementation exists; custom cryptography is not recommended.
- [ ] **Automatic Distributed Graph Traversal**: Manual CTE and bounded SurrealDB read-only GQL are separate foundations, not this capability.
- [x] **Qdrant / Redis Datastore (bounded)**: Separate capability APIs preserve vector and key-value semantics, enforce transport/resource limits and have live matrix evidence. They are not a universal Active Record backend or cluster/failover certification.
- [x] **Edge Databases (bounded v12)**: Direct official Hrana HTTP v3 support provides parameterized Turso SQL, atomic transactions, bounded responses/materialization, checksummed migrations and a real-SQL offline fallback; transparent replicas and SQLx Active Record parity are not claimed.
