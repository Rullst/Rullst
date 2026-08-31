# Why Rullst?

Rullst is an opinionated, Axum-based full-stack framework for teams that want
the productivity of a coordinated application platform without hiding Rust's
types or the underlying ecosystem.

Its strongest distinction is not that every individual feature is unique. It
is the combination of compile-time-oriented APIs, explicit security
boundaries, first-party backend capabilities, offline development contracts,
and one coordinated CLI and release train.

> **Version status:** Rullst v12 is under active development. This page lists
> implemented or explicitly bounded capabilities in the current source tree;
> it is not a production-readiness certificate. The
> [framework specification](spec.md),
> [capability ledger](capability-ledger.md), and
> [v12 release program](v12.md) are authoritative when a shorter description
> and the code disagree.

## The short answer

Choose Rullst when you want to build a backend-oriented Rust application with:

- an explicit Axum, Tokio, Tower, and SQLx foundation;
- compile-time-generated routes, HTML, and models, with compiler-visible
  diagnostics instead of runtime reflection;
- a coordinated ORM, authentication, security, jobs, mail, AI, billing,
  administration, observability, and CLI toolchain;
- secure, fail-closed defaults for privileged and production-facing surfaces;
- deterministic offline paths for supported external-provider integrations;
- server-rendered interfaces that do not require a project-local SPA bundle;
- standard escape hatches when a framework abstraction is not the right fit.

## What makes the combination distinctive

### 1. AI-native means explicit and inspectable

Rullst is designed so that both humans and coding agents can reason about an
application from its source. Common routes, models, HTML trees, policies, and
scaffolds are represented by typed Rust or macro input rather than runtime
class scanning or hidden reflection.

This does not make generated code automatically correct. It makes important
structure available to the compiler, review tools, and the CLI. Macro
diagnostics, compile-fail tests, generated-project gates, and the AST-based IDOR
scanner turn that structure into evidence.

The AI layer follows the same rule. Its bounded RAG pipeline makes tenant
context, retrieval, context limits, source metadata, and audit explicit in one
typed operation. It rejects differently tagged or unsafe passages and refuses
ungrounded generation when retrieval is empty. The included cosine retriever is
clearly process-local; production datastore authorization and durability are
not hidden behind an “automatic AI” claim.

Conversational memory follows that explicit model too: a static-dispatch
tenant-bound contract has a bounded offline store and an opt-in SQL adapter for
SQLite, PostgreSQL, MySQL, and MariaDB. Each successful turn commits the user
and assistant messages atomically, while revision compare-and-swap rejects
stale cross-process writers instead of silently scrambling history or
automatically repeating a billable model request.

### 2. A broad platform that keeps its foundations visible

Rullst coordinates a large backend surface in one versioned workspace, but it
does not replace its foundations with proprietary runtime primitives:

- applications can mount ordinary `axum::Router` values;
- middleware remains compatible with Tower composition;
- Tokio remains the asynchronous runtime;
- SQLx pools and raw parameterized queries remain available beside the ORM.

The [Axum and SQLx interoperability guide](axum-sqlx-migration.md) documents
the supported escape hatches. Framework conveniences can still require
migration work, so Rullst describes this as reduced lock-in rather than
"zero lock-in."

### 3. Security boundaries are part of framework design

The framework treats authorization and privileged tooling as architectural
inputs rather than deployment footnotes. Current bounded contracts include:

- fail-closed production environment and secret validation;
- CSRF, secure headers, request heuristics, DLP, abuse controls, and security
  telemetry primitives;
- route-scoped JSON Schema 2020-12 or OpenAPI 3.1-component enforcement with
  bounded offline compilation, local-only references and linear-time regexes;
- explainable aggregate threat assessment plus an opt-in authenticated,
  subject-bound, expiring and locally one-shot proof-of-work gate;
- Argon2id password hashing, expiring encrypted sessions, RBAC, model policies,
  and explicit owner-or-role guards;
- a typed OAuth/OIDC session transaction that keeps PKCE verifiers and OIDC
  nonces server-side, expires them after ten minutes, and consumes them before
  callback validation;
- an authenticated Nexus admin surface and a loopback/debug-constrained Studio;
- one bounded signed-payment-webhook verifier exposed through both Axum and
  opt-in Actix middleware adapters;
- versioned AES-256-GCM ORM field encryption with authenticated context;
- CI gates against `panic!`, `unwrap()`, and `expect()` in declared production
  targets.

These are defense-in-depth controls, not a claim that an application is secure
without its own authorization model, deployment controls, reviews, and tests.
The Sentinel is deliberately deterministic rather than marketed as autonomous
AI: the host still owns aggregate collection, identity, accessible fallback,
distributed replay state and enforcement.
See the [security architecture](security-architecture.md) and
[v12 security claims](v12-security-claims.md).

### 4. Persistence depth without a fictional universal database API

The relational ORM provides generated Active Record and query APIs,
transactions, migrations, relations, tenant scopes, policies, soft deletion,
auditing, encrypted fields, typed pgvector helpers, structured telemetry, and
an opt-in transactional outbox.

The primary relational matrix covers SQLite, PostgreSQL, MySQL, and MariaDB.
Turso/libSQL has a separate typed primary profile. MongoDB, DuckDB, SurrealDB,
Qdrant, Redis, and external search engines use capability-specific adapters so
their different consistency and query models stay visible.

This is deliberate: Rullst prefers several honest, bounded APIs over one API
that pretends documents, OLAP, graphs, vectors, key-value structures, and
relational transactions have identical semantics. Read the
[polyglot persistence](polyglot-persistence.md),
[transactional outbox](tutorials/38-transactional-outbox.md), and
[Scout search](tutorials/39-scout-search.md) guides.

### 5. External services remain usable during offline development

Supported AI, OAuth, mail, billing, search, and persistence adapters expose
deterministic offline behavior when configured with their documented empty or
`mock_*` credentials. This lets generated projects, tests, examples, and local
sandboxes run without silently contacting a third party.

The Brazilian fiscal boundary applies the same evidence-first approach: its
bounded DPS 1.01 builder, checksum-pinned official XSD catalogue, local
PKCS#12 XMLDSig verification, deterministic issuance JSON, strict
signed-authorization and structured-rejection codec, and mTLS client
preparation are testable without
calling SEFIN, while tax authorization remains explicitly disabled until the
external trust and homologation gates pass.

Mail scheduling follows the same bounded approach: SQLite and Redis persist a
due time and never claim it early, while unsupported real direct transports
reject a future message instead of delivering it immediately. Offline fixtures
may retain that timestamp for assertions. Polling delay,
at-least-once delivery, and provider acceptance remain visible operational
boundaries.

Mocks prove the local application contract, not the live provider contract.
Provider-specific production support still requires the applicable live or
protocol tests, credentials, policy, and operational evidence.

### 6. Framework upgrades are a product feature

`cargo rullst upgrade --dry-run` inventories a project and reports a versioned
migration plan. `cargo rullst upgrade` snapshots controlled files, updates the
coordinated Rullst dependency train, applies supported compiler fixes, runs a
Cargo check gate, and restores those files when the gate fails.

It intentionally refuses to invent database migrations, business rules, or
security policy. The useful distinction is a bounded, recoverable upgrade
transaction with human and machine-readable reports. See the
[assisted upgrade tutorial](tutorials/36-assisted-framework-upgrades.md).

### 7. Server-first UI with optional escalation

The compile-time `html!` macro escapes supported dynamic text and attribute
values, while `RawHtml` makes an unescaped boundary explicit. HTMX-oriented
server rendering is the default path for applications that do not need a
project-local SPA bundle.

LiveView, selected Wasm islands, Tera, and packaging helpers are optional
strategies with separate maturity and operational requirements. Rullst does
not claim that every frontend mode is interchangeable or bundle-free.

Omni extends that server-first model with a deterministic Tauri shell for
desktop, Android, and iOS instead of creating a second business backend. Its
portable versioned client envelope carries no local authority, and the opt-in
native offline foundation adds bounded idempotent proposals, explicit
conflict/resync state, account-bound encrypted snapshots, and a coordinator with
request budgets, timeouts, and cursor checks over application transports.
Platform secure storage, concrete HTTP/background transport, physical-device
behavior, signing, and store acceptance remain separate evidence gates; see the
[web-first Omni guide](tutorials/43-omni-web-first.md) and
[offline synchronization guide](tutorials/44-omni-offline-sync.md).

### 8. Local control surfaces use real application signals

Studio is a local developer control room with explicit unavailable states when
a probe is not connected. Nexus generates an authenticated administration
surface from registered model metadata and applies server-side field policy.
Studio's relational browser can also perform deliberately narrow primitive
row edits/deletions: the write surface exists only behind the verified local
request capability, binds values, identifies one row by its inspected complete
primary key and runs against four relational engines. It is not presented as a
replacement for application tenant authorization or production administration.
Studio flag toggles also invalidate already-warm database flag drivers in the
same process without an unbounded key registry; cross-process invalidation
remains an explicit application transport.

The queue monitor follows an explicit retention rule as well. SQLite removes
successful payloads by default, while applications that need operational
history can opt into a validated, atomically pruned limit and purge it from
Studio. Rullst does not silently trade privacy for a more impressive dashboard.

The ORM emits secret-free structured spans for generated and raw query
entrypoints, streams, transaction outcomes, and Rullst-owned pool acquisition
timing. The host owns subscriber initialization, sampling, collector security,
and retention. See the [telemetry guide](telemetry-guide.md).

### 9. Scaffolding is treated as shipped code

The CLI can generate projects, routes, models, migrations, auth, mail, billing,
deployment manifests, LMS foundations, and other bounded application slices.
Rullst tests representative generated projects and maintains structural
matrices instead of assuming that a template is correct because its source
file compiles inside the CLI crate.

The Academy slice is already more than a landing-page mock: its generated
SQLite journey exercises server-owned progress, assessment, score,
leaderboard, automation, notifications and school boundaries. Its accessible
lesson presentation supports bounded video/audio metadata, mandatory WebVTT
captions for video and escaped transcripts. That is a useful foundation for a
language-learning product, not a claim that Rullst generates pedagogy, content,
speech recognition, native-device behavior or a complete Duolingo equivalent.

For example, `make:billing --model Workspace` now materializes distinct SQLx
and Turso-primary persistence profiles. Both are generated, linted, migrated
and exercised through subscription ownership and collision negatives; the
test does not turn provider sandbox validation or distributed reconciliation
into an automatic claim.

The billing facade follows the same bounded design: an explicit generic
subscription handle delegates pause/cancel without erasing the provider type,
while its grace-period value validates time bounds but leaves persistence,
authorization and entitlement policy visible in application code.

The same CLI includes inspection, toolchain diagnostics, migration assistance,
SBOM generation, and a bounded static route-access scanner. Generated output
remains application code: review it, test it, and keep it under version control.

### 10. Ambition is separated from evidence

Rullst keeps ambitious ideas, but labels them as implemented, partial,
experimental, offline mock, roadmap, or not recommended. Benchmarks are
reported for their measured workload instead of becoming universal
performance slogans. Hardware, provider, regulatory, and certification claims
remain incomplete until the corresponding external evidence exists.

This distinction is essential for a broad framework. The
[capability ledger](capability-ledger.md) records the boundary; the
[technical comparison](comparatives.md) records both competitive strengths and
areas where other frameworks are more mature.

## Where Rullst is a particularly good fit

Rullst is worth evaluating for:

- backend-heavy SaaS and multi-tenant applications;
- server-rendered products that still need jobs, realtime, mail, billing, and
  administration;
- Rust teams that prefer one coordinated release train over assembling every
  application subsystem independently;
- projects that need relational data plus explicit document, analytics,
  search, vector, graph, or key-value capabilities;
- organizations that value offline reproducibility, secure scaffolds, and
  machine-readable engineering evidence;
- teams using coding agents that benefit from explicit APIs, bounded files,
  strong types, and a normative specification.

## When another choice may be better

Rullst may not be the best choice when:

- only a small HTTP router or middleware library is needed;
- a browser-component ecosystem is the primary product architecture;
- long production history, a very large plugin ecosystem, or commercial
  support matters more than Rust-native integration;
- the application requires a capability that the ledger still marks partial,
  experimental, or roadmap;
- the team does not want the maintenance surface of a batteries-included
  framework.

In those cases, using Axum directly, another Rust application framework, a
frontend-centered framework, or a mature platform in another ecosystem can be
the more responsible decision.

## How to evaluate Rullst today

1. Start with the [Zero-to-Hero tutorial](tutorials/01-hello-world.md).
2. Check the exact capability you need in the
   [specification](spec.md) and [capability ledger](capability-ledger.md).
3. Enable only the required Cargo features and inspect their dependency graph.
4. Run the documented tests against your database, provider, proxy, and threat
   model.
5. Pin an immutable release or commit for evaluation; do not deploy from a
   moving development branch.

This page will be reviewed again against the immutable v12 RC source and CI
evidence. Until then, it is a concise map of the strongest implemented ideas,
not a substitute for the detailed contracts.
