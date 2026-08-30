# Polyglot Persistence

Rullst v12 keeps explicit relational and specialized persistence contracts.
SQLx Active Record supports SQLite, PostgreSQL, MySQL, and MariaDB. The bounded
blank/API profile can instead use Turso/libSQL as its typed primary ORM.
MongoDB is for documents, DuckDB is for embedded OLAP/analytics, and SurrealDB
exposes documents plus bounded read-only graph queries. No optional adapter
silently changes the application's primary database.

## Choose the smallest feature

Using the umbrella crate:

```toml
[dependencies]
rullst = { version = "12.0.0", features = ["orm-mongodb"] }
```

Available umbrella features are `orm-turso`, `orm-mongodb`, `orm-duckdb`,
`orm-surrealdb`, and the convenience `orm-polyglot`. Direct `rullst-orm`
users select `turso`, `mongodb`, `duckdb`, `surrealdb`, or `polyglot`.

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
  --skip-initial-migration
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

```rust
let _ = dotenvy::dotenv();
rullst_orm::polyglot::TursoOrm::init_from_env().await?;

let mut event = Event {
    id: 0,
    label: "started".into(),
    active: true,
};
event.save().await?;
let current = Event::find(event.id).await?;
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

```rust
use rullst_orm::polyglot::{
    TursoConfig, TursoMigration, TursoQueryLimit, TursoStatement, TursoStore,
    TursoValue,
};

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

let collection = CollectionName::new("audit_events")?;
let id = DocumentId::new("event-2026-0001")?;
let page = DocumentPage::new(0, 50)?;
```

Collection names use a portable ASCII identifier grammar, IDs are bounded to
letters, digits, `_` and `-`, and every list requires a 1–500 row page. Models
must serialize as objects and must not own the driver ID field (`_id` for
MongoDB, `id` for SurrealDB); the portable `DocumentId` owns that concern.

### MongoDB

```rust
use rullst_orm::polyglot::{
    CollectionName, DocumentId, DocumentRepository, MongoDbStore,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct AuditEvent {
    action: String,
}

let store = MongoDbStore::<AuditEvent>::connect_or_mock(
    std::env::var("MONGODB_URL").unwrap_or_default(),
    "my_application",
).await?;

store.create(
    &CollectionName::new("audit_events")?,
    &DocumentId::new("event-1")?,
    &AuditEvent { action: "login".into() },
).await?;
```

An empty URL or one beginning with `mock_`/`mock://` selects an in-process,
deterministic `BTreeMap` store. A non-mock URL uses the official MongoDB Rust
driver. The release contract includes an optional Testcontainers CRUD suite;
availability, backups, indexes, topology and authorization remain deployment
responsibilities.

### SurrealDB documents and graph reads

```rust
use rullst_orm::polyglot::{
    GraphQuery, GraphRepository, SurrealAuth, SurrealConfig, SurrealDbStore,
};

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

```rust
use rullst_orm::polyglot::{
    AnalyticsRepository, AnalyticsValue, DuckDbStore, QueryLimit,
};

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
```

DuckDB is bundled for a predictable optional build. Its connection is guarded
and every native operation runs through Tokio's blocking worker pool. Dynamic
values are prepared parameters; SQL text remains trusted application structure.
Results must declare a 1–10,000 row materialization limit. Scalar, decimal,
temporal, interval, geometry and binary values are preserved; unsupported
complex result types fail with a typed error instead of being guessed.

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
