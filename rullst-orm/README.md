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

🚀 **[Visit the Official Website & Documentation Hub](https://rullst.github.io/Rullst/book/)** 🚀

Built on top of `sqlx` and procedural macros, **Rullst ORM** brings the delightful, fluent syntax of Active Record frameworks directly to the high-performance Rust ecosystem.

**Maturity: Core** ([tiers](https://rullst.github.io/Rullst/book/maturity.html)).
The optional store adapters (MongoDB, DuckDB, Turso, SurrealDB, Qdrant, Scout
search, Redis structures) document their own validation boundaries below.

<div align="center">
  <h3>🛡️ Security Engineering</h3>
  <p>Rullst ORM uses SQLx bindings, validated identifiers, typed errors, and layered CI checks. Workflow badges are scoped test results, not a guarantee for an application or deployment.</p>

| Security Audit | Status | Description |
| :--- | :---: | :--- |
| **OpenSSF Scorecard** | <a href="https://scorecard.dev/viewer/?uri=github.com/Rullst/Rullst"><img src="https://img.shields.io/badge/dynamic/json?url=https%3A%2F%2Fapi.scorecard.dev%2Fprojects%2Fgithub.com%2FRullst%2FRullst&query=%24.score&label=OpenSSF%20Scorecard&style=flat-square" alt="OpenSSF Scorecard" /></a> | Current public supply-chain practice score; not a security certification |
| **Release Provenance** | <a href="https://github.com/Rullst/Rullst/actions/workflows/release.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/release.yml?style=flat-square&label=" alt="Release provenance" /></a> | Provenance attestations for release artifacts; no SLSA level is claimed here |
| **Codecov** | <a href="https://codecov.io/gh/Rullst/Rullst"><img src="https://codecov.io/github/Rullst/Rullst/branch/main/graph/badge.svg?component=framework_libraries" alt="Framework library coverage" /></a> | Blocking 90% target for the measured framework-library scope; the complete repository aggregate also has its own 90% gate |
| **Matrix DB Tests** | <a href="https://github.com/Rullst/Rullst/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/ci.yml?style=flat-square&label=" alt="Testcontainers" /></a> | Live PostgreSQL, MySQL, MariaDB, MongoDB, SurrealDB and libSQL contracts, plus in-process DuckDB tests |
| **OpenSSF** | <a href="https://www.bestpractices.dev/projects/13359"><img src="https://img.shields.io/cii/level/13359?style=flat-square&label=" alt="OpenSSF Best Practices" /></a> | Open source security standards |
| **Property tests** | <a href="https://github.com/Rullst/Rullst/actions/workflows/proptest.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/proptest.yml?branch=main&style=flat-square&label=Proptest" alt="Proptest" /></a> | Scheduled/manual bounded invariant evidence |
| **Miri research matrix** | <a href="https://github.com/Rullst/Rullst/actions/workflows/miri.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/miri.yml?branch=main&style=flat-square&label=Miri" alt="Miri" /></a> | Manual bounded evidence; the selected privacy scope is strict, while native database FFI remains outside Miri |
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
  types at compile time; string-column methods validate identifiers at runtime,
  while explicitly named raw methods remain caller-owned SQL escape hatches.
  A column such as `raw` or `desc`, whose helper would repeat a fixed method
  (`where_raw`, `order_by_desc`), gets no generated helper; use
  `where_eq("raw", ...)` or `order_by("desc")` for it.
- **Eager Loading**: Batch supported `has_many`, `has_one`, `belongs_to`,
  `morph_many`, `morph_one`, and typed `morph_to` targets. Inverse polymorphic
  fields use an explicit target per relation and a persisted `<morph_name>_id`
  plus `<morph_name>_type` discriminator. A batch whose related rows exceed the
  global query cap fails with a `Validation` error instead of silently
  returning partial relations.
- **Fail-Closed Tenant Scopes**: Models declaring `tenant_column` require
  `with_tenant`, inject the tenant predicate (typed as the tenant field, so a
  mistyped context fails closed) into generated queries, protect
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
  [audit revision guide](https://rullst.github.io/Rullst/book/tutorials/50-auditable-revisions.html).
- **Bounded Post-Commit Effects**: `after_commit` and the generated observer
  `committed` callback run only after `Orm::transaction` or a direct generated
  save/delete/restore/force_delete commits. Rollback discards them, and post-commit failures use a
  distinct error that says the database is already durable. These callbacks
  remain process-local; caller-owned raw SQLx transactions cannot expose their
  eventual commit decision to generated hooks.
- **Durable Transactional Outbox**: opt-in `Outbox` events commit atomically
  with domain state and use stream-scoped idempotency keys, bounded leases,
  exact claim tokens, retries and dead-letter state. PostgreSQL, MySQL,
  MariaDB and SQLite share the contract. Delivery is at least once, so the
  external consumer must also be idempotent.
- **Data Governance & Privacy Helpers**: At-rest encryption, recursive audit masking, and data-erasure primitives; legal compliance remains application-specific.
  `#[orm(encrypted)]` and `#[orm(masked)]` values appear as `"***"` in generated
  `to_json()`, audit rows and committed events (including Redis
  `orm:events:*`), are omitted from Scout documents, and remain encrypted in
  `save_to_redis` hashes. `SecretString` fields get the same treatment, and
  `SecretString` itself serializes as an encrypted `RULLST:v2` envelope (never
  plaintext), which the query cache stores and decrypts on a hit. That
  envelope is not bound to a record, so deserialize client input with
  `#[serde(deserialize_with = "rullst_orm::privacy::deserialize_plaintext_secret")]`,
  which rejects envelopes instead of decrypting them. The key settings
  (`RULLST_ENCRYPTION_KEY`, `RULLST_ENCRYPTION_KEY_ID`,
  `RULLST_ENCRYPTION_KEYRING`) are read from the process environment, then
  from `./.env`, which is never loaded into the process environment.
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
- **Generated Embeddings**: the `ai` feature (which implies `pgvector`) adds
  `save_with_embedding(&rullst_ai::AiClient)` to models with an
  `#[orm(embedding_for = "...")]` vector field. The application must also
  depend on `rullst-ai`, whose client the generated method names.
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
  while explicit or task-scoped transactions are reused without nesting. The
  cascade honours the child's tenant scope, not its `global_scope`.
- **Transactional Partial Updates (v13 candidate)**: `.update_partial()` merges
  selected values into the locked current row and runs the full save lifecycle,
  including audit and post-commit effects. The loaded row runs `after_fetch`
  first, like `find()`. It refreshes the caller model after
  success and offers `save_with_tx`; the SQL, rollback and migration notes
  are in [Transactional partial updates](https://rullst.github.io/Rullst/book/transactional-partial-updates.html).
  Use it, or a `RowSnapshot` field that makes `save()` write only the changed
  columns, when requests may edit the same row at the same time; see
  [concurrent updates and counters](#concurrent-updates-and-counters).
- **Native Relational Enums**: `#[derive(Enum)]` owns one closed label mapping
  for SQLx, Serde and ORM values. `Blueprint::native_enum` emits a named,
  drift-checked PostgreSQL type with `strict-postgres`, inline MySQL/MariaDB
  `ENUM`, or a SQLite `TEXT CHECK` constraint.
- **Stable Keyset Chunking**: `.chunk_by_id()` traverses ascending generated
  `i32` IDs without offset drift when already processed rows are deleted, and
  propagates callback errors. `.chunk()` remains available for offset-based
  compatibility and orders its pages by the primary key unless the query
  sets `order_by`.
- **Transaction-Aware Redis Query Cache**: `.remember(seconds)` uses a
  versioned SHA-256 key bound to the application namespace, the active tenant
  (for a model with a `tenant_column`; other models share one global entry),
  generated SQL and typed bindings. Generated reads bypass cache inside every
  ORM transaction so Redis cannot replace the transaction's database view.
  Generated model saves/deletes/restores/force-deletes invalidate the table's
  global keys and, for a tenant-scoped model, those of the tenant active at
  the write, only after commit (even when that
  `with_tenant` scope ended before the commit) through a per-table key index,
  never a keyspace `SCAN`, so write latency does not grow with unrelated keys
  in a shared Redis database; cluster/failover evidence remains outside the
  current contract.
- **Model Policies (Authorization)**: `#[orm(policy = "MyPolicy")]` checks generated
  instance mutations. Policy-protected models reject `delete_all()` because
  bulk SQL cannot invoke per-row authorization; load the intended rows and call
  their instance `delete()` methods inside `Orm::transaction` instead.
- **Development lazy-loading diagnostics**: An opt-in development policy can fail loudly when a guarded lazy load would hide an N+1 query.
- **Explicit Capability Boundaries**: Unsupported replication paths fail closed instead of reporting simulated success.
- **Optional Polyglot Persistence**: Feature-gated MongoDB document CRUD,
  DuckDB OLAP queries, Turso/libSQL edge SQL, and SurrealDB document/read-only
  graph operations live behind explicit capability APIs instead of pretending
  to be one universal Active Record interface. See the
  [Polyglot guide](https://rullst.github.io/Rullst/book/polyglot-persistence.html).
- **Authenticated Portable Document Recovery**: MongoDB, SurrealDB and the
  deterministic store expose stable identifier-preserving inventory. A bounded
  AES-256-GCM snapshot performs two equal source scans, binds application and
  collection scope, resumes only into an empty or exactly matching subset and
  verifies the final inventory. It is an application-operated recovery
  primitive, not an online cross-store transaction or managed backup service.
- **Bounded Turso Primary Profile**: `#[derive(Orm)]` with
  `#[orm(backend = "turso")]` exposes typed CRUD/query methods through
  `TursoOrm`, reversible checked migrations, and a persistent offline or remote
  libSQL store. The CLI offers this primary path for the Blank and API
  starters (`cargo rullst new --database turso`); SQLx-specific relations,
  hooks, schema auto-diff and the SaaS starter are not claimed as parity.

---

### Query and transaction boundaries

`select`, `pluck_string`, and `pluck_i32` accept validated column identifiers,
including `table.column`. Expressions and aliases belong in explicit raw SQL
APIs. Qualified names retain skipped/encrypted-field restrictions; supported
encrypted string projections use the original field name when decrypting.

`where_in` and `or_where_in` with an empty vector emit a false predicate, so an
empty selected-ID list cannot become an unrestricted read or delete. An empty
`where_not_in` remains true. Unfiltered `delete_all` on models without policies
is still deliberately a bulk operation. Its statement renders only the WHERE
and soft-delete predicates, so an explicit `limit()`, `offset()`, `order_by()`,
a join, `group_by()` or a CTE makes it fail with `Validation` instead of
deleting every matching row; select the IDs first and delete them with
`where_in("id", ids)`. Model-wide, tenant, and soft-delete
scopes constrain every user `OR` branch, and keyset traversal applies its cursor
to the entire original filter. Nested generated subqueries propagate validation
errors, including a missing tenant context, into their containing query.
Typed subqueries passed to `where_exists`, `or_where_exists`, `with_cte` and
`with_recursive` are embedded with portable `?` markers; on PostgreSQL the
final statement is numbered once, in textual order, so nested scopes, CTEs and
joins keep every tenant and caller binding at its own `$n` position.
Caller-owned raw CTEs and select lists pass their values with
`with_raw_bindings`, `with_recursive_raw_bindings` or `select_raw_bindings`;
those values are bound at the fragment's own position (before JOIN, scope and
WHERE values) and a marker/binding count mismatch fails with `Validation`.
`with_raw`, `with_recursive_raw` and `select_raw` accept only fragments without
bind markers: `bind()` supplies WHERE values, so a marker before FROM would
otherwise receive the tenant or scope binding while the mandatory predicate
received the caller's value.
Only PostgreSQL statements are renumbered: `delete_all()`, the child `UPDATE`
issued by `cascade_soft_delete`, and instance `restore()`/`force_delete()`
receive `$n` there and keep `?` markers on MySQL/MariaDB and SQLite.
The renumbering reads quoted text with PostgreSQL's default
`standard_conforming_strings = on` rules: a backslash escapes only inside
`E'...'` strings, so a raw `ESCAPE '\'` does not hide later markers.
Generated `Model::search()` uses those same model-wide and tenant scopes for
both its SQL fallback and external Scout result IDs. The SQL fallback never
matches `#[orm(hidden)]`, `#[orm(encrypted)]`, `#[orm(masked)]` or
`SecretString` columns, treats `%` and `_` in the query literally, and rejects
queries over 1,024 bytes or with control characters. Missing tenant context
fails before contacting Scout, and an empty provider result remains an empty
match even when a database contains an explicitly inserted ID of zero. Search
index access controls still belong to the application/operator. Providers
return at most `scout::MAX_SEARCH_HITS` (1,000) IDs for the whole shared index,
before tenant, model-wide and soft-delete scopes apply; for a model with any of
those scopes an answer of that size is treated as truncated and `search()`
uses the SQL fallback instead, so another tenant's hits cannot hide a tenant's
matches (the fallback's substring semantics then apply).

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

Prefer `Orm::transaction` with ordinary model/query methods when combining
eager relationships or `after_fetch` hooks with transactional reads. Fetches
release the managed transaction lock before invoking hooks and loading related
records, and scalar projections use that same transaction. Caller-owned raw
SQLx transactions still support plain `get_with_tx` reads, but eager-loading or
`after_fetch` configurations fail with an actionable error because their
secondary queries require the managed executor context.

A nested `Orm::transaction` joins the active transaction through a savepoint
instead of opening a second pooled transaction. An error inside it rolls back
only the nested work; a success commits (or rolls back) with the outer
transaction, and its `after_commit` callbacks run after the outer commit. A
helper that wraps `Outbox::enqueue` or model saves in its own
`Orm::transaction` is therefore atomic with its caller. The returned future is
`Send`, so it can also run in a spawned task. Do not hold the shared handle's
lock across a nested call.

Sibling nested transactions started concurrently on one task (for example with
`tokio::join!`) take turns on the shared connection: each opens its savepoint
only after the previous sibling's savepoint was released or rolled back, so a
failure rolls back only that sibling's work. Plain model statements issued by
a sibling future while another sibling's savepoint is open still run inside
that savepoint; wrap each concurrent branch in its own `Orm::transaction` when
they must be isolated. A savepoint that cannot be settled (for example a nested
future cancelled while another operation holds the connection) makes the
enclosing transaction fail closed: it rolls back and returns an error instead
of committing, and the pool closes any connection returned while still inside
a transaction instead of reusing it.

A transaction-backed stream retains exclusive access to the transaction until
it is consumed or dropped. Consume/drop it before starting another operation
on that transaction: while a `stream()` opened inside `Orm::transaction` (or
`#[rullst_orm::test]`) is alive, other generated ORM calls on that transaction,
including those of a `tokio::join!` sibling, fail with `Validation` instead of
waiting for its lock. Transactional streams reject `after_fetch` hooks to avoid
reentrant queries while retaining that access; use `get()` inside
`Orm::transaction` for those models. Streaming does not eagerly load relations.

`force_delete()` and `restore()` run in a savepoint like `delete()`.
`force_delete()` runs the delete hooks and observers, writes a `force_deleted`
audit entry on auditable models (so it needs an `AuditContext` there) and
registers the same post-commit cache, Redis, `committed` and Scout removal
effects; it does not cascade. `restore()` re-reads the row, calls the `updated`
and `saved` observers, writes a `restored` audit entry and registers the update
effects of `save()`, including a Scout re-index. Their `can_force_delete` and
`can_restore` policies run before the transaction, as before.

A `save()` update, `delete()` or `force_delete()` that matches no row (the row
was deleted after the handle was loaded) returns `RecordNotFound` (the
tenant-scope `Validation` error on tenant models) and runs no later observer,
audit or post-commit effect. Deleting a trashed row again fails the same way
without touching its deletion time, and restoring a missing or live row is a
no-op. If the COMMIT of a direct `save()` fails, the handle keeps its previous
`id`, so retrying the save inserts the row again.

Only `delete()`, `restore()` and `force_delete()` change the soft-delete marker
of an existing row. `save()` leaves that column out of its `UPDATE` (an
`INSERT` still writes it), so saving a handle loaded before `delete()` edits
the trashed row instead of silently undeleting it without `can_restore`, the
`restored` audit or restore observers. An auditable save copies the stored
marker into the handle so its audit entry records no false change, and
`update_partial()` rejects a soft-delete value with `Validation`.

Full `save`/`delete` policies and their lifecycle callbacks run while their
executor is borrowed.
Ordinary ORM calls made recursively from those callbacks on the same task fail
with a typed `Validation` error instead of waiting on their own transaction.
Perform database authorization and any required row locking explicitly within
the managed transaction before invoking the mutation; the policy can then
validate trusted context and in-memory model values. Defer only post-commit
effects to `after_commit`/`committed`, which run outside that callback scope.
Failure in an `after_save` hook still rolls back the generated mutation. Inside
an existing transaction (`save_with_tx`, or `save()` within `Orm::transaction`)
the save runs in a savepoint, so catching that error and committing the outer
transaction does not persist the failed write.
Independently spawned tasks and separately retained raw connections do not
inherit this executor contract and must not be used to bypass atomicity.

Raw SQL, unscoped `QueryBuilder::new()` construction, direct editing of builder
SQL fields, and custom `SubqueryBuilder`
implementations remain trusted application code. Custom subqueries should
expose their validation failure through `validation_error()`; the trait's
compatibility default does not validate arbitrary custom SQL. These controls
do not authenticate users or decide who may invoke `unscoped()`.

## 🛠️ Quick Start

### Installation

Install the v13 prerelease with an exact pin (Cargo never selects a
prerelease on its own). The v12 line (`rullst-orm@12`) reached end of life on
16 October 2026:

```bash
cargo add rullst-orm@=13.0.0-alpha.4 -F strict-sqlite
cargo add tokio -F full
cargo add sqlx@0.9 --no-default-features
```

`#[derive(FromRow)]` names `sqlx`, so the application depends on it
directly; `rullst-orm` enables the SQLx features it needs, and your own `sqlx`
entry only has to be the same 0.9 release. Since 13.0.0-alpha.4 the code that
`#[derive(Orm)]` generates reaches `tracing` and `tokio` through `rullst_orm`,
so neither needs a dependency of its own; `tokio` above runs the example's
`#[tokio::main]`. A project generated by `cargo rullst new` already lists
these dependencies.

Select the driver of your database: `strict-sqlite`, `strict-postgres` or
`strict-mysql`, or `drivers-all` for all three on SQLx's dynamic `Any` pool.
Since 13.0.0-alpha.3 the crate enables no driver by default; without one,
`Orm::init` fails at startup with an error that names the feature to enable.

### Zero-to-Hero Example

```rust
use rullst_orm::schema::Schema;
use rullst_orm::{FromRow, Orm};

// 1. Just add the Orm macro to your struct!
#[derive(Debug, Clone, FromRow, Orm)]
pub struct User {
    pub id: i32, // ID = 0 means it hasn't been saved yet
    pub name: String,
    pub email: String,
    #[orm(hidden)] // Excluded from generated to_json(), audit, event and search projections
    pub password: String,
}

#[tokio::main]
async fn main() -> Result<(), rullst_orm::Error> {
    // 2. Initialize the connection pool (SQLite, Postgres, MySQL/MariaDB)
    Orm::init("sqlite::memory:").await?;

    // 3. Create the `users` table (a migration does this in an application)
    Schema::create("users", |table| {
        table.id();
        table.string("name");
        table.string("email");
        table.string("password");
    })
    .await?;

    // 4. Create a new user through the generated Active Record API
    let mut user = User {
        id: 0,
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
        password: "secret_password".to_string(),
    };
    
    user.save().await?; // Runs INSERT and hydrates the generated ID.

    // 5. Fluent Queries
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
  column keeps full-row saves), `auth`, `make:billing`,
  `make:chat-session` and the models of every `cargo rullst new` starter
  (Blank, API and SaaS). Their struct literals set
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
`i32`, `i64`, `u32`, `f64`, `bool`, the `chrono` `DateTime<Utc>`,
`NaiveDateTime`, `NaiveDate` and `NaiveTime` types, a `#[derive(Enum)]` type,
and `Option` of those. `None` binds SQL
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

### Selecting an isolated SQLx driver

A standalone ORM dependency compiles only the SQLx drivers it selects:

```toml
rullst-orm = { version = "=13.0.0-alpha.4", default-features = false, features = ["strict-postgres"] }
```

`strict-mysql` and `strict-sqlite` select their respective backends, and
`drivers-all` the three-driver convenience profile. Up to 13.0.0-alpha.2
`drivers-all` was the default feature; since alpha.3 there is no default
driver, so `default-features = false` only matters for older versions.
`Orm::driver()` reads the SQL dialect from the DSN scheme, ignoring case:
`postgres://` and `postgresql://` are PostgreSQL, and `mysql://` and
`mariadb://` are MySQL/MariaDB. IPv6 literal hosts use brackets
(`postgres://app@[2001:db8::10]:5432/app`); every `Orm::init*` entrypoint
rejects any other bracketed text, such as an unedited `[your-database-id]`
template placeholder, before connecting.
If an application previously disabled defaults without choosing a backend,
enable `drivers-all` explicitly or select a strict backend when upgrading.

Cargo features combine across dependencies. A dependency enabling
`drivers-all` (as Studio's default features do), a SQLite queue or Turso's
offline transport can restore other drivers;
the example above proves a standalone ORM boundary, not a complete facade or
Studio application. Check the final normal/build graph with `cargo tree`.
Derived enums also emit codecs only for the ORM's selected drivers; the
application does not need matching Cargo feature names. The CI consumer compiles
generated CRUD, transactions and enum encoding/decoding and rejects unrelated
SQLx driver packages separately from the workspace test matrix.

### Native database enums

Select a strict primary feature in generated applications. PostgreSQL native
enums specifically require `strict-postgres`, because SQLx's dynamic `Any`
driver cannot decode custom PostgreSQL types:

```toml
rullst-orm = { version = "=13.0.0-alpha.4", features = ["strict-postgres"] }
```

Then derive one label contract and use it in schema code:

```rust
use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{Enum, Orm};

#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq)]
#[rullst_enum(type_name = "account_status", rename_all = "snake_case")]
enum AccountStatus {
    AwaitingReview,
    Active,
}

async fn create_schema() -> Result<(), rullst_orm::Error> {
    Orm::init("postgres://user:password@localhost/application").await?;
    Schema::create("accounts", |table: &mut Blueprint| {
        table.id();
        table.native_enum::<AccountStatus>("status").not_null();
    })
    .await?;
    Ok(())
}
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

### Constraints, indexes and dates

The `Blueprint` declares `Column::unique()`, table-level `unique(&[...])`,
`index(&[...])`, `unique_index(&[...])`, foreign keys
(`foreign("user_id").references("users", "id").on_delete(ForeignKeyAction::Cascade)`,
or `foreign_id("user_id", "users")` to add the column too) and a validated
`check("...")` of trusted developer SQL. PostgreSQL and SQLite create indexes
with `CREATE INDEX IF NOT EXISTS` after the table; MySQL/MariaDB declare them in
`CREATE TABLE` and cannot index a `TEXT` column, so give indexed strings a
`VARCHAR` `col_type`. `Schema::table("posts", |table| ...)` adds columns,
indexes and constraints to an existing table; SQLite changes it cannot make
(adding a foreign key to an existing column, a CHECK) return
`Error::UnsupportedSchemaChange` before any statement runs.

`date()`, `time()`, `timestamp()` and `timestamp_tz()` hold
`chrono::NaiveDate`, `NaiveTime`, `NaiveDateTime` and `DateTime<Utc>` fields
(`rullst_orm::chrono`): native `DATE`/`TIME`/`TIMESTAMP`/`TIMESTAMPTZ` on
PostgreSQL, `DATE`/`TIME(6)`/`DATETIME(6)` (UTC) on MySQL/MariaDB and ISO-8601
text that sorts by time on SQLite. `table.timestamps()` adds nullable
`created_at`/`updated_at` `timestamp_tz()` columns that default to the current
time, and `#[orm(db_default)]` leaves a field to its database default. Those
columns and fields need a strict driver feature: the SQLx `Any` pool cannot
read date/time values, so there the native helpers fail on PostgreSQL and
MySQL/MariaDB and `timestamps()` keeps `TEXT DEFAULT CURRENT_TIMESTAMP`
columns for `String` fields (`DEFAULT (CURRENT_TIMESTAMP)` on MySQL/MariaDB,
MySQL 8.0.13+, MariaDB 10.2.1+). See the
[ORM guide](https://rullst.github.io/Rullst/book/crates/orm.html#constraints-indexes-and-foreign-keys)
for every DDL form.

`table.float(...)` and `table.boolean(...)` map to `f64` and `bool` model
fields. PostgreSQL receives `DOUBLE PRECISION` and `BOOLEAN` (an integer
`ColumnDefault` of `0`/`1` becomes `FALSE`/`TRUE`); MySQL/MariaDB receive
`DOUBLE` and an `INTEGER` 0/1 flag; SQLite keeps `REAL` and `INTEGER`. This
applies to newly built DDL only: PostgreSQL columns created by earlier versions
remain `REAL`/`INTEGER` until a reviewed migration alters them, and a model that
paired `boolean()` with an integer field on PostgreSQL must switch to `bool`
(or use `integer()`) for new tables.

### Optional Redis query cache

Enable the `redis` feature and give each application sharing a Redis database a
stable namespace. Generated Redis methods and effects follow this ORM feature;
the application does not need a `redis` feature of its own:

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

Generated `save_to_redis`/`get_from_redis`/`increment_redis_field` hashes are
keyed by that namespace, the table and, for tenant models, an opaque digest of
the active tenant: they require `with_tenant(...)` and never read or overwrite
another tenant's hash. For a model without a tenant scope, a hash stored by
12.0 or 12.1 under `orm:<table>:<id>` is still read while the namespaced hash
is missing and moves to the namespaced key on its next `save_to_redis` or
`increment_redis_field`. Tenant models never read that shared key; migrate
their hashes explicitly as described in the
[Redis guide](https://github.com/Rullst/Rullst/blob/main/docs/src/redis-guide.md#model-hashes-stored-by-120-and-121).

An explicitly remembered query outside a transaction requires Redis
initialization. Connection/command failures and corrupt cache entries fall back
to the database, while missing configuration fails closed. Explicit and
task-scoped transactions always bypass the cache. Generated model saves and
deletes invalidate that table's generated cache keys after commit, using the
index each cache write maintains for its table (at most 10,000 live keys per
write; members of entries that already expired are pruned and not counted, and
entries cached by earlier versions are not read again and expire by TTL). Raw SQL,
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
```

`Outbox::install()` is only an explicit setup/test helper. Register
`OutboxMigration` through the application's normal migration runner in
production. Streams and event keys are case-sensitive on every backend; on
MySQL/MariaDB, run `Outbox::install()` once (for example from a new migration)
to convert a table created by an earlier version, whose key columns used the
server's case-insensitive default collation. Generated observers are not silently persisted, and an ACK lost
after the external effect can cause redelivery; see the
[transactional outbox tutorial](https://rullst.github.io/Rullst/book/tutorials/38-transactional-outbox.html).

---

## 📚 Documentation

The [ORM guide](https://rullst.github.io/Rullst/book/crates/orm.html) in the
[Rullst Book](https://rullst.github.io/Rullst/book/) covers models, relations,
schema, migrations and the optional stores in more depth.

---

## 🛡️ Security

Rullst ORM uses SQLx prepared-statement bindings for values accepted by its query builders. Structural identifiers are restricted to a bounded ASCII identifier grammar before interpolation. Raw SQL and application authorization remain the caller's responsibility; these controls reduce injection risk but are not an absolute safety guarantee.

## 📄 License
This project is licensed under the [MIT License](https://github.com/Rullst/Rullst/blob/main/LICENSE).
