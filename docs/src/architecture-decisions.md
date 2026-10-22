# Architecture choices in generated projects

`cargo rullst new` records product capabilities that materially change the
generated application. Version 12 intentionally exposes one audited application
architecture instead of presenting incomplete compatibility markers as equal
implementations. The [framework specification](spec.md) and
[capability ledger](capability-ledger.md) remain authoritative.

## ORM style

Version 12 database-backed projects are generated with Active Record. The CLI
does not ask for a global ORM architecture and does not expose an `--orm` flag.
The framework's repository APIs remain available when an application needs an
explicit persistence boundary.

| Architecture | v12/v13 direction | Trade-off |
| --- | --- | --- |
| Active Record | The single generated v12 profile and a retained v13 option | Concise, but persistence concerns remain close to models. |
| Data Mapper / Repository | Application-owned in v12; planned as a separately complete v13 profile | More explicit persistence boundary and more code to maintain. |
| Hybrid | Not a global project profile | The two styles may naturally coexist per module, but the application must define that boundary. |

Earlier v12 prerelease menus offered three labels, but Repository and Hybrid
shared generation branches and support varied by blueprint. Removing those
selectors does not remove ORM APIs; it prevents the generator from claiming
equivalent end-to-end profiles before the generated routes, services and tests
support them consistently. All database operations still need parameter
binding, authorization and transaction design.

For most CRUD-oriented applications, start with Active Record and introduce a
repository around domains that genuinely need a separate persistence boundary.

## Database engines and capabilities

The first database selector chooses exactly one SQL Active Record backend:

| Selection | Current v12 boundary |
| --- | --- |
| SQLite | Local SQLx pool, migrations and relational ORM contract. |
| PostgreSQL | SQLx PostgreSQL pool and live container CRUD/schema contract. |
| MySQL | SQLx MySQL pool and live MySQL CRUD/schema contract. |
| MariaDB | The same MySQL protocol implementation with a separate live MariaDB contract. |
| Turso | Typed Turso/libSQL primary profile for the blank full-stack or API starter; SQLx-specific blueprints do not offer it. |

The Blank starter can instead have no primary database (`--no-database`, or
**No database** in the v13 wizard).

A second multi-select adds zero or more independent capabilities and accepts
`Enter` with no selection. Capabilities already selected by the primary profile
or CLI flags are omitted. In the v13 wizard the same screen also offers AI,
Redis, a Dockerfile and a Nix flake. Turso supplies explicit edge SQL,
transactions and checked migrations; MongoDB supplies portable document CRUD; DuckDB supplies
bounded analytics; SurrealDB supplies document CRUD and bounded read-only graph
queries; Qdrant supplies bounded dense-vector operations. Selecting one adds the
precise Cargo features and environment keys, but does not make every model
portable between different database families.

This split prevents a Turso, MongoDB, analytics, or graph label from generating
an application whose SQL Active Record migrations cannot run. See the
[Polyglot Persistence guide](polyglot-persistence.md) for the APIs and limits.

## Frontend profile

Version 12 generates server-rendered `html!` views with HTMX enhancement and
does not ask for a frontend engine or expose a `--frontend` flag. The five labels
shown by earlier prerelease wizards were not five complete, interchangeable
renderers:

| Selection | Current v12 boundary |
| --- | --- |
| HTMX + Tailwind SSR | Audited default scaffold using server-rendered `html!` views. There is no project-local SPA bundle, but HTMX and styles are still browser assets that must be pinned and served. |
| LiveView | Core supplies `LiveComponent`, `Live::mount` and `live_ws_handler`. The application registers routes and supplies the HTMX WebSocket extension, authentication and reconnect policy. |
| Wasm Island | `#[island]`, `make:island` and `build:client` provide a dual-target hydration foundation. Asset delivery, CSP, state, routing and browser E2E remain application work. |
| Pico.css | Records the compatibility profile; the application must add, pin and serve Pico.css and validate the resulting pages. |
| Tera | Adds the Tera dependency in the generated manifest. A complete file-template renderer and migration of every blueprint view are not generated automatically. |

Version 13 should model independent capabilities instead of one misleading
global frontend selector: rendering (SSR or API), interaction (HTMX, LiveView or
Wasm islands), styling (the bundled design or an explicitly materialized CSS
system), and templating (`html!` or a fully generated file-template path). A
combination should appear only after its assets, routes and browser behavior
have equivalent tests.

## Full-stack versus headless API

- Full-stack blueprints include server-rendered routes and relevant local tool
  integration.
- `--api` generates a JSON-oriented Blank project without HTML view rendering;
  product blueprints reject it instead of silently retaining their HTML routes.

Generated code is application code. Review its routes, auth boundaries,
database schema and dependencies before deployment.

## Optional AI

Selecting AI adds `rullst-ai` and its provider-agnostic client foundation.
Providers still require explicit credentials or the documented deterministic
mock mode. Ollama can keep prompt traffic on infrastructure you operate, but
“local” alone does not establish an air gap. Measure compile time, binary size,
latency, quality and cost for the chosen provider and model.

## Cache and queues

Without Redis, Rullst offers process-local memory cache and local queue options.
With Redis selected, the generated project can configure the feature-gated
Redis adapters.

There is deliberately no silent production fallback from Redis to memory: a
distributed deployment that loses Redis must not pretend that independent
process-local state is equivalent. Choose the backend explicitly and test its
failure policy. Core realtime broadcast/presence remains in-process; Redis cache
and queue support does not turn it into distributed WebSocket pub/sub.

## Studio and Nexus

- Studio's supported v12 launcher is debug-only, binds to loopback and verifies
  direct peer information. It shows only data supplied by real local probes.
- Nexus uses a local-development policy or explicit release credentials. A
  dependency or generated route does not remove the need for application RBAC.

Neither interface should be exposed publicly without an application-owned
identity, authorization, TLS and network policy.

## Installable applications

Rullst uses a **web-first** model. The web application is the canonical,
universally reachable product; the server remains authoritative for domain
rules, authentication, authorization, persistence, realtime and security.

`cargo rullst make:pwa` makes that application installable as a Progressive
Web App: a web app manifest, placeholder icons, `/sw.js` and an `/offline`
page, wired into the starter's router and layouts. The service worker works
under the default nonce Content-Security-Policy and never stores a non-GET
request or a response marked `no-store` or `private`, which Rullst answers
by default, so authenticated pages are not kept on the device. The same site
reaches Google Play as a Trusted Web Activity (a Bubblewrap configuration and
`/.well-known/assetlinks.json` from `rullst::pwa::AssetLinks`) and the
Microsoft Store through PWABuilder; iOS users add it from Safari. See the
[PWA guide](tutorials/43-progressive-web-apps.md).

Until 13.0 the Omni command packaged the site in Tauri shells for desktop,
Android and iOS. Those shells deliberately exposed no native API to the
site, so they gave the same result as an installed PWA while costing three
release workflows, Tauri upgrades and per-platform signing. 13.0 removes
`make:omni`, `cargo rullst omni` and the native-only `offline-sync` feature,
whose only consumer was the shell.

`rullst::client_contract` supplies one shared `rullst.client` v1 JSON boundary
for browser, server and future platform-enhanced code. It negotiates positive
versions, rejects unknown outer fields and oversized bodies, and carries typed
payloads, correlation, optional mutation idempotency, server time and bounded
failure codes. It deliberately has no user/tenant/role claim: authentication,
authorization, domain validation and durable replay handling remain server
work. A page that accepts writes offline queues them in IndexedDB with an
idempotency key, as the PWA guide shows; the server accepts each key once.

## Engineering invariants

Repository policy requires typed failures rather than production `panic!`,
`unwrap()` or `expect()`, explicit SQL parameterization/sanitization, and the
workspace test/Clippy/format gates. These are review and CI rules, not a claim
that arbitrary application code or third-party dependencies cannot panic.

Rullst exposes Axum/Tower integration so applications can add middleware and
routes directly. That escape hatch also means the final security and
observability composition belongs to the application.

## Practical starting points

- Single-process CRUD application: use the generated Active Record, `html!` SSR
  and HTMX profile with memory cache.
- Domain-heavy service on v12: introduce repositories per module with explicit
  transaction boundaries. In v13, select a Data Mapper / Repository project
  profile only after its blueprint has full generated-route and test parity.
- Multi-process application: a configured shared cache/queue plus deployment
  tests; do not assume realtime is distributed.
- Rich browser interaction: opt into LiveView or Wasm foundations only with
  application-owned integration and browser tests until a complete v13 profile
  exists.

Revisit these choices as evidence changes. Do not infer performance, security or
availability from a wizard label.
