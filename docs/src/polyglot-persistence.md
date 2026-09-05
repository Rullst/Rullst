# Polyglot Persistence

> [!IMPORTANT]
> Dependency examples use `12.0.0-rc.1`, the planned first v12 RC. Do not
> request it from crates.io before it is published; use path dependencies from
> this source checkout during development.

Rullst v12 keeps explicit relational and specialized persistence contracts.
SQLx Active Record supports SQLite, PostgreSQL, MySQL, and MariaDB. The bounded
blank/API profile can instead use Turso/libSQL as its typed primary ORM.
MongoDB is for documents, DuckDB is for embedded OLAP/analytics, SurrealDB
exposes documents plus bounded read-only graph queries, Qdrant exposes dense
vector search, and Redis exposes selected native structures. No optional adapter
silently changes the application's primary database.

## Choose the smallest feature

Using the umbrella crate:

```toml
[dependencies]
rullst = { version = "12.0.0-rc.1", features = ["orm-mongodb"] }
```

Available umbrella features are `orm-turso`, `orm-mongodb`, `orm-duckdb`,
`orm-surrealdb`, `orm-qdrant`, `orm-redis`, and the convenience
`orm-polyglot`. Direct `rullst-orm` users select `turso`, `mongodb`, `duckdb`,
`surrealdb`, `qdrant`, `redis`, or `polyglot`.

Features are additive. Prefer one adapter unless the application genuinely
uses several, and inspect the final dependency graph with:

```bash
cargo tree -e features
```

The project wizard first asks for a primary backend. SQLite, PostgreSQL, MySQL,
and MariaDB use SQLx; Turso is available as a typed primary for the blank/API
starter. The wizard then offers explicit additive capability adapters. For a
hybrid SQLx application, use one or more explicit flags:

```bash
cargo rullst new edge_app --default --database mariadb --turso --mongodb \
  --qdrant --skip-initial-migration
```

MariaDB shares the SQLx MySQL protocol implementation but has its own live
container contract. `--turso` is additive; `--database turso` is the explicit
primary selection and currently rejects SQLx-specific non-blank blueprints.

## Turso / libSQL as the primary ORM

```bash
cargo rullst new edge_app --default --api --database turso
cd edge_app
cargo rullst make:model Event --migration
cargo rullst db:migrate
```

The generated manifest enables `orm-turso` without a direct SQLx dependency,
and the environment contains `TURSO_DATABASE_URL`, `TURSO_AUTH_TOKEN`, and
`TURSO_OFFLINE_PATH` rather than a fictitious `DATABASE_URL`. Models use the
same public derive with an explicit backend:

```rust
#[derive(Debug, Clone, rullst_orm::Orm)]
#[orm(table = "events", backend = "turso")]
struct Event {
    id: i64,
    label: String,
    active: bool,
}
```

Initialize once before using the generated inherent methods:

```rust,no_run
# #[derive(Debug, Clone, rullst_orm::Orm)]
# #[orm(table = "events", backend = "turso")]
# struct Event {
#     id: i64,
#     label: String,
#     active: bool,
# }
# async fn use_turso_model() -> Result<(), Box<dyn std::error::Error>> {
rullst_orm::polyglot::TursoOrm::init_from_env().await?;

let mut event = Event {
    id: 0,
    label: "started".into(),
    active: true,
};
event.save().await?;
let current = Event::find(event.id).await?;
# let _ = current;
# Ok(())
# }
```

The typed contract includes CRUD, equality filters, ordering, bounded
pagination/counts, app-assigned or generated primary keys, checksummed
migrations, status, and rollback. Generated `make:model` and `make:migration`
commands retain the Turso backend. It does not yet provide SQLx ORM relations,
hooks, automatic timestamps, seed generation, schema auto-diff, or transparent
embedded-replica synchronization. Those limits are why only the blank/API
starter is currently advertised as Turso-primary. The derive rejects
unsupported SQLx-specific model behaviors at compile time instead of silently
ignoring their attributes.

## Turso / libSQL explicit edge SQL

```rust,no_run
use rullst_orm::polyglot::{
    TursoConfig, TursoMigration, TursoQueryLimit, TursoStatement, TursoStore,
    TursoValue,
};

async fn use_edge_sql() -> Result<(), Box<dyn std::error::Error>> {
let config = TursoConfig::new(
    std::env::var("TURSO_DATABASE_URL").unwrap_or_default(),
    std::env::var("TURSO_AUTH_TOKEN").unwrap_or_default(),
);
let edge = TursoStore::connect(config).await?;
let create = TursoStatement::new(
    "CREATE TABLE events (id INTEGER PRIMARY KEY, label TEXT NOT NULL)",
    vec![],
)?;
edge.migrate(vec![TursoMigration::new("m20260829_events", vec![create])?])
    .await?;

edge.execute(TursoStatement::new(
    "INSERT INTO events VALUES (?1, ?2)",
    vec![TursoValue::Integer(1), TursoValue::Text("started".into())],
)?).await?;
let rows = edge.query(
    TursoStatement::new(
        "SELECT id, label FROM events WHERE id = ?1",
        vec![TursoValue::Integer(1)],
    )?,
    TursoQueryLimit::new(100)?,
).await?;
let _ = rows;
Ok(())
}
```

The live path speaks the official Hrana HTTP v3 protocol directly through the
workspace's Rustls-backed HTTP client; it does not embed the native SQLite
engine or the legacy remote SDK dependency chain. URLs must use
`libsql://` or HTTPS; cleartext HTTP requires an explicit loopback-development
opt-in. Requests reject redirects and have a 30-second deadline. Tokens and
endpoint details are redacted from `Debug`; remote responses are capped at
16 MiB, result materialization at 1–10,000 rows, each statement at 1,024
positional parameters/8 MiB of parameter data, and each transaction at 1,024
statements/16 MiB of raw SQL plus parameters. SQL result cells can still be
large, so applications must also constrain selected columns and data at the
schema/query boundary.

An empty or `mock_*` endpoint selects a single-connection SQLite in-memory
fallback. It executes real SQL and migrations deterministically but does not
simulate remote replication, latency, failover, or Turso Cloud. The live CI
contract runs the same API against the official `sqld` container and proves
that a failed multi-statement batch rolls back its earlier writes.

## The portable document contract

MongoDB, SurrealDB, and the deterministic offline store implement the same
bounded document operations:

```rust
use rullst_orm::polyglot::{
    CollectionName, DocumentId, DocumentPage, DocumentRepository,
};

# fn bounded_document_inputs() -> Result<(), Box<dyn std::error::Error>> {
let collection = CollectionName::new("audit_events")?;
let id = DocumentId::new("event-2026-0001")?;
let page = DocumentPage::new(0, 50)?;
# let _ = (collection, id, page);
# Ok(())
# }
```

Collection names use a portable ASCII identifier grammar, IDs are bounded to
letters, digits, `_` and `-`, and every list requires a 1–500 row page. Models
must serialize as objects and must not own the driver ID field (`_id` for
MongoDB, `id` for SurrealDB); the portable `DocumentId` owns that concern.

### Encrypted export and crash-resumable restore

MongoDB, SurrealDB and the deterministic store also implement
`DocumentInventory<T>`, which retains each validated identifier. That enables
one deliberately conservative recovery path without pretending the engines
share transactions:

```rust,no_run
use rullst_orm::polyglot::{
    CollectionName, DocumentRecoveryBinding, DocumentRecoveryKey,
    DocumentRecoveryPolicy, export_document_snapshot,
    restore_document_snapshot,
};

# async fn recovery<S, D, Event>(source: &S, destination: &D) -> Result<(), Box<dyn std::error::Error>>
# where
#     S: rullst_orm::polyglot::DocumentInventory<Event>,
#     D: rullst_orm::polyglot::DocumentInventory<Event>,
#     Event: serde::Serialize + serde::de::DeserializeOwned + Send + Sync,
# {
let collection = CollectionName::new("audit_events")?;
let binding = DocumentRecoveryBinding::try_new(
    "my_application.production",
    collection,
)?;
let key = DocumentRecoveryKey::try_new(
    "recovery-2026-09",
    [42_u8; 32], // load 32 random bytes from a secret manager
)?;
let policy = DocumentRecoveryPolicy::try_new(100, 10_000, 16 * 1024 * 1024)?;

// Persist this opaque envelope in application-owned durable storage.
let snapshot = export_document_snapshot(source, &binding, &key, policy).await?;
let report = restore_document_snapshot(
    destination,
    &snapshot,
    &binding,
    &key,
    policy,
).await?;
assert_eq!(report.verified(), report.inserted() + report.replayed());
# Ok(())
# }
```

The key uses AES-256-GCM and a fresh nonce; its authenticated data binds the
rotation ID, trusted application namespace and exact collection. Key material
passes through a zeroizing temporary and key/snapshot `Debug` output is
redacted. The policy permits pages of 1–500, 1–100,000 documents and a 1 KiB–64
MiB plaintext ceiling. Models in this portability path must be JSON objects
without either `id` or `_id`.

Export scans twice and refuses to seal unequal observations. This detects
ordinary concurrent changes, but it is not a database snapshot isolation
primitive: pause writers or use a source-side transaction/export facility.
Restore accepts only an empty destination or a matching subset left by an
earlier attempt. It inserts without replacement, rejects different or extra
rows before mutation and verifies an exact final inventory. A failed attempt
can retain earlier successful inserts, so retry the same authenticated
snapshot. Keep unrelated destination writers paused until verification. The
destination database, namespace and collection/table must already be
provisioned where the engine requires them; schema or permission failures stay
visible and are never reclassified as an empty destination.

The live release matrix exercises MongoDB → SurrealDB → MongoDB. It proves the
bounded adapter contract on that runner, not backup retention, key custody,
replication, point-in-time recovery, topology failover or vendor operations.

### MongoDB

```rust,no_run
use rullst_orm::polyglot::{
    CollectionName, DocumentId, DocumentRepository, MongoDbStore,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct AuditEvent {
    action: String,
}

async fn store_audit_event() -> Result<(), Box<dyn std::error::Error>> {
let store = MongoDbStore::<AuditEvent>::connect_or_mock(
    std::env::var("MONGODB_URL").unwrap_or_default(),
    "my_application",
).await?;

store.create(
    &CollectionName::new("audit_events")?,
    &DocumentId::new("event-1")?,
    &AuditEvent { action: "login".into() },
).await?;
Ok(())
}
```

An empty URL or one beginning with `mock_`/`mock://` selects an in-process,
deterministic `BTreeMap` store. A non-mock URL uses the official MongoDB Rust
driver. The release contract includes an optional Testcontainers CRUD suite;
availability, backups, indexes, topology and authorization remain deployment
responsibilities.

### SurrealDB documents and graph reads

```rust,no_run
use rullst_orm::polyglot::{
    GraphQuery, GraphRepository, SurrealAuth, SurrealConfig, SurrealDbStore,
};

async fn read_graph() -> Result<(), Box<dyn std::error::Error>> {
let config = SurrealConfig::new(
    std::env::var("SURREALDB_URL").unwrap_or_default(),
    "main",
    "application",
    SurrealAuth::bearer(
        std::env::var("SURREALDB_TOKEN").unwrap_or_default(),
    ),
);
let store = SurrealDbStore::<serde_json::Value>::connect_or_mock(config)?;
let query = GraphQuery::read_only(
    "MATCH (person:person)-[knows:knows]->(friend:person) RETURN friend",
    100,
)?;
let rows = store.query_graph(&query).await?;
let _ = rows;
Ok(())
}
```

The adapter uses SurrealDB's documented HTTP `/key`, `/sql`, and `/gql`
protocol instead of embedding its BSL-licensed SDK. It disables redirects,
requires HTTPS outside loopback unless cleartext is explicitly enabled, redacts
authentication in `Debug`, streams through a configurable 1 KiB–8 MiB memory
ceiling, and sends namespace/database headers on every scoped request.

Graph queries must start with `MATCH`; semicolons, caller-provided `LIMIT`, and
the `INSERT`, `SET`, `REMOVE`, and `DELETE` tokens are rejected. The adapter
then appends its own 1–1,000 row limit. This is a conservative read boundary,
not arbitrary SurrealQL execution or a graph mutation API. The `/gql` endpoint
requires SurrealDB 3.2 or newer and explicit experimental enablement on the
3.2 release line; the live contract pins 3.2.4 and enables that capability.

## DuckDB analytics

```rust,no_run
use rullst_orm::polyglot::{
    AnalyticsRepository, AnalyticsValue, DuckDbStore, QueryLimit,
};

async fn analyze_events() -> Result<(), Box<dyn std::error::Error>> {
let analytics = DuckDbStore::in_memory().await?;
analytics.execute(
    "CREATE TABLE events (sequence BIGINT, label VARCHAR)",
    vec![],
).await?;
analytics.execute(
    "INSERT INTO events VALUES (?, ?)",
    vec![
        AnalyticsValue::Signed(1),
        AnalyticsValue::Text("started".into()),
    ],
).await?;
let rows = analytics.query(
    "SELECT sequence, label FROM events WHERE sequence >= ?",
    vec![AnalyticsValue::Signed(1)],
    QueryLimit::new(500)?,
).await?;
let _ = rows;
Ok(())
}
```

DuckDB is bundled for a predictable optional build. Its connection is guarded
and every native operation runs through Tokio's blocking worker pool. Dynamic
values are prepared parameters; SQL text remains trusted application structure.
Results must declare a 1–10,000 row materialization limit. Scalar, decimal,
temporal, interval, geometry and binary values are preserved; unsupported
complex result types fail with a typed error instead of being guessed.

## Specialized Qdrant and Redis stores

Qdrant has a separate dense-vector contract rather than a document or Active
Record facade. See [RAG Systems & Vector Search](tutorials/22-rag-vector-search.md)
for its bounded collection, upsert, delete and cosine-query API. The project
wizard and deterministic CLI expose it as the additive `--qdrant` choice.

Redis native structures are also explicit and namespaced:

```rust,no_run
use rullst_orm::{
    RedisDataConfig, RedisDataKey, RedisDataStore, RedisField, RedisMember,
    RedisScanLimit, RedisStructuresRepository, RedisValue,
};

# async fn redis_example() -> Result<(), Box<dyn std::error::Error>> {
let config = RedisDataConfig::new(
    std::env::var("REDIS_URL").unwrap_or_default(),
    "my-application",
    std::env::var("REDIS_USERNAME").unwrap_or_default(),
    std::env::var("REDIS_PASSWORD").unwrap_or_default(),
);
let data = RedisDataStore::connect_or_mock(config).await?;
let account = RedisDataKey::new("account:42")?;
data.hash_set(
    &account,
    &RedisField::new("display_name")?,
    &RedisValue::new("Ada")?,
).await?;
data.set_add(&account, &RedisMember::new("reader")?).await?;
let roles = data
    .set_scan(&account, RedisScanLimit::new(100)?)
    .await?;
# let _ = roles;
# Ok(())
# }
```

The adapter also supplies atomic signed hash increments, exact membership,
Sorted Set add/top ranking and exact per-structure deletion. Empty or `mock_*`
endpoint/ACL credentials choose the deterministic fallback. Live remote Redis
requires `rediss://`; intentionally unauthenticated development uses the
loopback-only constructor. Hash values are capped at 1 MiB, members at 4 KiB,
and scans/ranges at 1–1,000 accepted rows. Redis may still allocate a protocol
value before client-side validation, so untrusted writers require server ACLs,
quotas and isolation. Lists, Streams, cluster/failover, eviction and durable
Pub/Sub are not part of this datastore contract.

## What this feature does not promise

- no cross-backend transaction, replication, migration, or consistency layer;
- no claim that document databases implement SQL joins or Active Record;
- no automatic choice of a datastore based on a model;
- no managed backups, production credentials, provider homologation, indexes,
  cluster failover, or performance guarantee;
- no universal “support for every database”.

Run the application's exact feature set in addition to the full workspace
gates. Remote production readiness still requires a disposable live environment,
negative authorization tests, backup/restore rehearsal, and operational review.
