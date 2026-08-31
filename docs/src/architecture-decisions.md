# Architecture choices in generated projects

`cargo rullst new` records several project choices. This page distinguishes
code that the current generator materially changes from compatibility profiles
that still require application integration. The [framework specification](spec.md)
and [capability ledger](capability-ledger.md) remain authoritative.

## ORM style

Projects with a database can select Active Record, Repository, or Hybrid.

| Selection | Generated intent | Trade-off |
| --- | --- | --- |
| Active Record | Model-oriented CRUD helpers | Concise, but persistence concerns remain close to models. |
| Repository | Repository modules in supported blueprints | More explicit persistence boundary and more code to maintain. |
| Hybrid | Both styles may coexist | Useful for incremental adoption, but teams need a convention per domain. |

These selections alter scaffold output; they do not convert an existing domain
model automatically or guarantee strict Domain-Driven Design. All database
operations still need parameter binding, authorization and transaction design.

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

A second multi-select adds independent capabilities. Turso supplies explicit
edge SQL, transactions and checked migrations; MongoDB supplies portable
document CRUD; DuckDB supplies bounded analytics; SurrealDB supplies document
CRUD and bounded read-only graph queries. Selecting one adds the precise Cargo
features and environment keys, but does not make every model portable between
different database families.

This split prevents a Turso, MongoDB, analytics, or graph label from generating
an application whose SQL Active Record migrations cannot run. See the
[Polyglot Persistence guide](polyglot-persistence.md) for the APIs and limits.

## Frontend profile

The current wizard exposes five selections, but they are not five complete,
interchangeable renderers:

| Selection | Current v12 boundary |
| --- | --- |
| HTMX + Tailwind SSR | Audited default scaffold using server-rendered `html!` views. There is no project-local SPA bundle, but HTMX and styles are still browser assets that must be pinned and served. |
| LiveView | Core supplies `LiveComponent`, `Live::mount` and `live_ws_handler`. The application registers routes and supplies the HTMX WebSocket extension, authentication and reconnect policy. |
| Wasm Island | `#[island]`, `make:island` and `build:client` provide a dual-target hydration foundation. Asset delivery, CSP, state, routing and browser E2E remain application work. |
| Pico.css | Records the compatibility profile; the application must add, pin and serve Pico.css and validate the resulting pages. |
| Tera | Adds the Tera dependency in the generated manifest. A complete file-template renderer and migration of every blueprint view are not generated automatically. |

The selector is therefore an architectural starting point, not proof that the
resulting application has a production-ready frontend. Use HTMX SSR when you
want the most exercised path in this repository. Select another profile only
when you intend to own and test its integration.

## Full-stack versus headless API

- Full-stack blueprints include server-rendered routes and relevant local tool
  integration.
- `--api` generates a JSON-oriented project and skips the interactive frontend
  selection.

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

## Omni packaging

Rullst uses a **web-first, platform-enhanced** model. The web application is the
canonical universally reachable product; the server remains authoritative for
domain rules, authentication, authorization, persistence, realtime and
security. Omni may package that interface and later add scoped native
capabilities, but it must not fork the business model or trust a client-side
decision as authority.

`cargo rullst make:omni` scaffolds a Tauri-powered shell and commands for
desktop and mobile targets. Deterministic generation pins the local Tauri CLI,
requires an explicit validated backend URL and application-owned identifier for
mobile, derives or validates product metadata, generates real platform icon
assets and fails when an explicitly requested platform cannot be initialized.
The packaged bootstrap exposes no remote IPC. A native navigation policy allows
only Tauri's local origin and the configured backend's exact origin, leaving
OAuth/external links for a reviewed system-browser/deep-link contract.

Path-aware workflows generate disposable applications and check desktop on
Linux/macOS/Windows, an Android debug APK and an iOS simulator target. Their
configuration is not proof until each hosted run passes on the referenced SHA.

That gate proves reproducible generation and simulator compilation only. It
does not automatically implement offline synchronization, secure remote
networking, platform signing, privacy declarations, physical-device behavior,
store publication, native updater policy or every frontend profile. Push,
biometrics, OS secure storage, deep links and offline state must be opt-in,
least-privilege additions with platform tests. Treat the generated shell as
application-owned packaging code that must be reviewed, signed and tested on
each supported target.

## Engineering invariants

Repository policy requires typed failures rather than production `panic!`,
`unwrap()` or `expect()`, explicit SQL parameterization/sanitization, and the
workspace test/Clippy/format gates. These are review and CI rules, not a claim
that arbitrary application code or third-party dependencies cannot panic.

Rullst exposes Axum/Tower integration so applications can add middleware and
routes directly. That escape hatch also means the final security and
observability composition belongs to the application.

## Practical starting points

- Single-process CRUD application: Active Record, HTMX SSR and memory cache.
- Domain-heavy service: Repository or Hybrid, with explicit transaction
  boundaries.
- Multi-process application: a configured shared cache/queue plus deployment
  tests; do not assume realtime is distributed.
- Rich browser interaction: opt into LiveView or Wasm foundations only with
  integration and browser tests in the application.

Revisit these choices as evidence changes. Do not infer performance, security or
availability from a wizard label.
