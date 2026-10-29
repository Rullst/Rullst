# Rullst Studio 📊

`rullst-studio` is the built-in, local-first administration and monitoring
dashboard for Rullst. It exposes bounded database, queue and in-process
telemetry views from the sources explicitly supplied by the application.

**Maturity: Extension** ([tiers](https://rullst.github.io/Rullst/book/maturity.html)):
optional and supported for its documented scope, a local developer control
room. The 13.x stability promise begins with stable `13.0.0`.

## ✨ Features

- **Database inspector:** Read and filter configured SQLx tables, edit bounded
  primitive non-key values, delete one primary-key-selected row with explicit
  confirmation, and inspect a live ER diagram. Mutation contracts run against
  SQLite, PostgreSQL, MySQL, and MariaDB; PostgreSQL runs under both the
  default `sqlx::Any` build and `strict-postgres`.
- **API reference:** Render the operations of an `OpenApi` document explicitly
  supplied by the application at `/studio/api` (server-side, no script or
  remote asset) and serve the document at `/studio/api/openapi.json`; Studio
  does not infer arbitrary Axum routes or send requests for you.
- **Worker queue monitoring:** Inspect up to 50 records exposed by a supplied
  Rullst queue and request retries. Records come from the bounded v13
  `list_job_previews` projection: at most 2 KiB of each payload and error
  leaves the SQLite or Redis store, and previews cut there are marked `…`.
  SQLite removes successful jobs, so the view is not durable completion
  history.
- **Safe configuration view:** Environment values are deny-by-default redacted;
  typed runtime configuration is projected without URLs, paths, or secrets.
- **Feature flags manager:** Toggle database-backed flags and immediately
  invalidate already-warm `DbFeatureDriver` caches in the same process. Studio
  never creates the `rullst_feature_flags` table; when it is missing, the page
  shows the schema to add in a migration.
- **Distributed diagnostics:** Visualize local spans plus bounded,
  attribute-free v1 spans submitted through a separately mounted
  HMAC-authenticated push endpoint. The profiler reports slow SQL labels and a
  possible N+1 heuristic without accepting SQL text, bindings, headers, bodies,
  attributes, or error strings.
- **Cache inspector:** An explicitly supplied memory or Redis `Cache` exposes
  at most 100 metadata records. The UI renders opaque HMAC identifiers, value
  sizes and TTLs and can invalidate one entry; it never renders logical keys or
  values and does not offer bulk flush.
- **Local-first security:** The supported launcher binds to loopback, verifies
  the direct peer and local `Host` authority, and requires same-origin `Origin`
  on mutations.
- **Self-contained interface:** Studio embeds its compiled stylesheet and its
  small request-stream client. Its database, cache, environment, trace and ER
  pages do not depend on a browser-side Tailwind compiler, Google Fonts, HTMX,
  Mermaid, or a public CDN.

## 🚀 Quickstart

Add `rullst-studio` to your project. This README describes the v13
prerelease; Cargo never selects a prerelease on its own, so pin it exactly
(the v12 line, `rullst-studio@12`, reached end of life on 16 October 2026):

```bash
cargo add rullst-studio@=13.0.0-alpha.4
```

### Launching the Studio

The supported mode is a standalone debug server. `run_studio` serves it on
`http://127.0.0.1:5555/studio` (this example also needs `tokio` with its
`macros` and `rt-multi-thread` features):

```rust,no_run
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    rullst_studio::run_studio(5555_u16).await
}
```

`run_studio` and
`Studio::into_router(LocalStudioAccess::loopback_only())` reject release builds
and requests whose direct peer is not verified as loopback (an IPv4-mapped
peer such as `::ffff:127.0.0.1` from a dual-stack `::` listener counts as IPv4
loopback). Servers composing
the router manually must preserve Axum `ConnectInfo<SocketAddr>`. Non-local
`Host`, cross-origin requests, and unsafe requests without `Origin` fail closed.
Studio responses use `Referrer-Policy: same-origin` so that browsers keep the
real origin on Studio's own form posts. `Origin: null` is accepted only with
`Sec-Fetch-Site: same-origin` (sent when a host layer imposes `no-referrer`);
a bare `null` origin or a same-site document on another port is rejected.
Data-browser writes, queue retry/purge and feature-flag toggles additionally
require a crate-private marker created only by that verified access middleware,
so importing the raw browser, `jobs_monitor` or `feature_flags` router cannot
turn those handlers into an unprotected write API; without the marker they
return `403`.

### Remote projects (v13, since 13.0.0-alpha.3)

The `rullst-studio` binary (`cargo install rullst-studio --version =13.0.0-alpha.4 --features remote --locked`, then
`rullst-studio [--port PORT]`, or `cargo rullst studio --remote`; library
users enable the non-default `remote` feature) serves
Studio on `127.0.0.1:5556` without an application process and reads the
read-only `/_rullst/monitor/v1` document of the projects you add: overview,
routes, queues and N+1 pages with a project switcher. It keeps the loopback
`Host`, same-origin and marker checks above, adds a per-process form token,
and runs in release builds because it serves no local data: the data
browser, `.env` viewer, AI and API playgrounds and migration pages are not
mounted. Projects are stored owner-only in the user's configuration
directory, with each token AES-256-GCM encrypted under a key in the data
directory and bound to its name and base URL. Requests require `https://`
(or `http://` to loopback), follow no redirects, ignore proxy variables and
read at most 1 MiB in 5 seconds. See the
[production monitoring guide](https://github.com/Rullst/Rullst/blob/main/docs/src/production-monitoring.md#watch-your-projects-from-studio).

History and alerts: projects marked as watched are polled every 60 s while
the binary runs; their aggregates go to a bounded owner-only SQLite file
(seven days) charted on `/studio/history`, and per-project rules (unreachable,
5xx rate, p95, pending jobs, queue failures) fire with a cool-down on
`/studio/alerts`, optionally posted once to an encrypted `https://` webhook.

**Database selection:** Studio uses the process-wide ORM pool that the
application initialized (`Server`, Artisan or an explicit `Orm::init`). When no
pool exists yet, the first database view initializes it once with the resolver
shared by `Server` and Artisan: the process `DATABASE_URL`, then `DATABASE_URL`
from `./.env` (which never overrides the process environment), then
`[database].url` parsed from `./Rullst.toml`. Without a configured database,
Studio reports that its database tools are unavailable and creates nothing;
there is no `sqlite://db.sqlite` fallback. Error messages never echo
configuration content. An application that calls `Orm::init` with its own URL
should do so before Studio serves requests. `data_browser::resolve_db_url`
remains only for API compatibility; Studio no longer uses it.

The earlier `StudioLayer` embedded-production idea was never implemented.
Keeping an authenticated shared Studio is worthwhile, but it needs its own
explicit identity/RBAC/TLS policy before it can become a supported mode.

**Since 12.1.0:** the raw data-browser router includes the same-origin
CSS/client and an unconnected Cache page, including when nested under
`/studio`; `12.0.0` omits those routes. An unconnected cache is
reported as **Unavailable**, not fabricated telemetry. Applications upgrading
from 12.0 must remove temporary asset/Cache `.route(...)` overrides to avoid
duplicate-route collisions; preserve their access-control layers. See the
[migration checklist](https://github.com/Rullst/Rullst/blob/main/docs/src/3-rullst-studio.md#v1210-browser-composition-fix).

**CLI Launch:**

Inside a project, the CLI runs the application with the `studio` argument
(`cargo run -- studio`):

```bash
cargo rullst studio
```

Core's Artisan handler then serves a smaller local compatibility UI (database,
AI, telemetry, security, capital and span pages) on `127.0.0.1:5555`, not the
`rullst-studio` router described here.

### Distributed trace ingestion

Create one bounded store, give it to the local viewer, and separately mount the
push-only ingestion router on the application endpoint reachable by trusted
producers:

```rust,no_run
use rullst_studio::distributed_traces::{
    DistributedTraceStore, TraceIngestionKey, TraceIngestor,
};
use rullst_studio::{LocalStudioAccess, Studio};

# fn app() -> Result<(), Box<dyn std::error::Error>> {
let store = DistributedTraceStore::new(2_048)?;
let key = TraceIngestionKey::new(std::env::var("RULLST_TRACE_INGESTION_KEY")?)?;
let ingestion = TraceIngestor::new(store.clone(), "api-1", key)?;

// Mount this push-only router in the application. TLS, network policy, key
// distribution, rotation and endpoint availability remain deployment work.
let _application_ingestion = ingestion.router();

// The viewer still requires the debug/loopback boundary.
let _studio = Studio::new()
    .with_distributed_traces(store)
    .into_router(LocalStudioAccess::loopback_only())?;
# Ok(())
# }
```

Each `TraceIngestor` binds one exact producer name to one key; use separate
endpoints/keys over the same store for multiple producers. Producers use
`TraceBatchSigner::new("api-1", key)` to obtain the exact body and four headers.
The receiver accepts 1–128 spans in at most 128 KiB, verifies HMAC-SHA256, a
60-second clock window and a one-time nonce, validates the complete batch, and
then commits idempotently. The store is process-local and bounded; it is not an
OTLP collector, durable trace backend, key manager, or production Studio login.

## 🔐 Security Audit

`rullst-studio` currently supports verified-loopback development access. It does
not provide a built-in shared-intranet or production authentication mode. Do not
expose raw subrouters publicly; a future shared mode must fail closed behind
application-owned authentication, administrator authorization, TLS, and network
policy.

Migration actions remain CLI-driven because standalone Studio has no migration
registry. Queue, revenue, security, AI, and telemetry views expose only supplied
process-local sources; unsupported operations or disconnected integrations are
reported explicitly.

The SSE request view records method, URI, status and latency only. It does not
capture bodies or headers by default because those can contain authentication,
session, payment, and personal data. A successful Studio toggle invalidates all
already-warm `DbFeatureDriver` entries in the same process. Other processes and
direct database writers remain visible through the configured TTL unless the
application supplies distributed invalidation.

The trace ingestion endpoint is a distinct push-only router and contains no
Studio read or mutation routes. Keep the HMAC key in a secret manager, rotate
it through application deployment, synchronize producer clocks, use TLS and
network policy, and do not put personal data or secrets in operation labels.
The cache inspector uses the same verified local request marker as database
mutations for individual invalidation. Its HTML contains neither cache values
nor exact logical keys.

Table views load at most 25 rows per page, ordered by the complete primary key
(otherwise by every selected column). The database cuts each cell's text to 256
characters before Studio renders it; key columns keep up to 16 KiB for row
actions, and a longer key makes its row read-only. Search terms are limited to
256 bytes and match the displayed columns (at most 256); the record count uses
the same predicate.

The table view marks SQL NULL as `NULL` and a present value it cannot decode as
text, a number or a Boolean (such as a non-UTF-8 BLOB) as `unreadable`, so a
filled column is never reported as NULL.

Data-browser mutation forms use database-inspected tables, columns and complete
primary keys; on PostgreSQL they cover the `public` schema, which every data
statement names explicitly. SQL values are parameterized; only text, signed integer, finite
float and Boolean codecs are writable, so MySQL/MariaDB unsigned or zero-filled
integers stay read-only. Primary keys and backend-specific types
remain read-only, request bodies are limited to 64 KiB, and deletion requires
typing `DELETE <table>`. A row offers only the actions whose form (for Edit,
with a maximum-size value) fits that limit; an empty text key is addressable. A table stays read-only when any primary-key column is
outside Studio's ASCII identifier boundary or beyond its 256-column cap, or
uses a floating-point type whose rendered text is rounded. A row whose key value
is `NULL`, is not decodable as text or contains a line break or NUL (which
browsers rewrite in form values) offers no actions. Each write runs in a database
transaction that commits only when exactly one row changed; otherwise it is
rolled back and reported as `404` (no row) or `409` (several rows). Storage
engines without transactions, such as MySQL MyISAM, cannot provide that
rollback. This is a local developer database tool, not application
authorization, tenant policy, audit history, undo, or a supported
shared-production admin surface.

## 📚 Documentation

For supported usage and security boundaries, see the
**[Rullst Book](https://rullst.github.io/Rullst/book/)** and its
capability ledger. A production/shared Studio mode is not currently supplied.
