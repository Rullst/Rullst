<!-- “Antes de gerar qualquer coisa, leia e siga estritamente o arquivo da verdade abaixo. spec.md”  -->

# Rullst Specification 📄
### *"The Single Source of Truth (SST) for Framework Architecture & Conventions"*

This document is the **Single Source of Truth (SST)** for the **Rullst Framework**. It specifies the exact conventions, API structures, naming rules, directory standards, and subsystem maturity lifecycles across all monorepo crates.

> [!IMPORTANT]
> **AI & Human Alignment Directive:**
> Whenever updating, refactoring, or generating code/documentation for Rullst, **always** refer to this specification as the baseline. 
> Every capability in the framework is strictly tagged with its implementation lifecycle status:
> - 🟢 **`[Implemented / Bounded]`**: A defined implementation exists with automated tests for the stated scope. This is not a deployment, provider-homologation, or certification claim.
> - 🟠 **`[Partial]`**: Useful foundations exist, but a named interoperability, architecture, or conformance boundary is still incomplete.
> - 🟡 **`[Offline Test Mock / Simulador Dev]`**: Deterministic offline sandbox fixtures for local development and offline CI without external API dependencies.
> - 🔵 **`[Roadmap / Em Construção]`**: Architectural design, public traits, and domain models specified in full, with production drivers in active engineering.

---

## 📂 1. Directory Structure Conventions

A standard Rullst application scaffold strictly adheres to this folder hierarchy:

```text
my-app/
├── src/
│   ├── controllers/      # Route controllers (async request handlers)
│   │   └── mod.rs
│   ├── models/           # Active Record & Repository Models (rullst-orm entities)
│   │   └── mod.rs
│   ├── pages/            # Shared HTML views, templates, and layouts
│   │   └── mod.rs
│   ├── middlewares/      # Custom application middleware layers
│   │   └── mod.rs
│   └── main.rs           # Application entrypoint, server bootstrap & central routing
├── Cargo.toml            # Project cargo dependencies
└── Rullst.toml           # Framework configuration (database, environment, secrets)
```

---

## 🛠️ 2. Naming Conventions

To guarantee consistency, both humans and AI coders must adhere to the following naming normalization rules:

* **File Names:** Standard Rust `snake_case` (e.g. `users_controller.rs`, `post_model.rs`, `billing_service.rs`).
* **Struct / Model / Trait Names:** Standard `PascalCase` (e.g. `UsersController`, `PostModel`, `PaymentProvider`).
* **URL Paths:** Lowercase kebab-case (e.g. `/users`, `/user-profiles`, `/billing/webhooks`).
* **Database Identifiers:** Snake case (e.g. `user_id`, `created_at`, `billing_accounts`).

---

## ⚡ 3. Framework Crates & Capability Matrix

| Crate | Responsibilities | Status & Capabilities |
| :--- | :--- | :--- |
| **`rullst-core`** | Kernel HTTP runtime, `routes!`, Server bootstrap, HTML engine, async task queues, WebSockets, circular telemetry buffers, storage facade, and the default baseline CSRF/WAF/header/PII stack. | 🟢 **`[Implemented / Bounded]`**: Routing, server lifecycle, `html!` engine, graceful shutdown, backpressure guard, queues, and local storage with path-traversal protection. SQLite and Redis persist `dispatch_at` timestamps for at most 366 days and never claim them early; Redis promotion uses server time and a digest-pinned live CI/release contract. Execution starts on the first later worker poll and is at-least-once. Custom drivers fail closed for future scheduling until implemented. `TenantStorage`, `TenantCache`, `TenantRealtime` and `TenantPresence` bind those facades to a validated `TenantContext`, apply immutable tenant namespaces and prove same-name local non-interference; the realtime wrappers also bound channel/event/identity names and payload size. Remote bucket policy, distributed transport/liveness and application room authorization remain deployment/application work.<br/>🟢 **`[Implemented / Bounded]`**: The in-memory upload admission contract enforces a hard size/allowlist boundary, canonical tenant/name, recognized signature versus MIME/extension, active-text denial, randomized tenant quarantine keys, SHA-256 binding and fail-closed scanner release. It is not multipart streaming, a deep parser, remote persistence or a production malware engine.<br/>🟢 **`[Implemented / Bounded]`**: Validated environment precedence is `RULLST_ENV`, legacy `APP_ENV`, then `[app].env`; invalid values fail instead of silently enabling development.<br/>🟢 **`[Implemented / Bounded]`**: `apply_security_baseline` and `Server` compose configured CSP nonce headers, exact-origin CORS with explicit credential opt-in, bounded WAF, double-submit CSRF and optional PII masking in one tested order, with the per-application config installed outside every middleware. Browser/proxy/TLS deployment evidence and application-owned session/auth/tenant/authorization remain separate. A fail-closed typed Academy boundary-assessment contract records those application observations without certifying them, and the extended `rullst-security` stack is still composed explicitly.<br/>🔵 **`[Roadmap]`**: Native S3/R2 direct cloud drivers. |
| **`rullst-orm`** | Active Record & Repository patterns, parameterized SQLx connection pool (PostgreSQL, MySQL/MariaDB, SQLite), typed Turso/libSQL primary profile, schema migrations, AES-256-GCM privacy, Scout search, typed pgvector/Qdrant queries, Redis native structures, and optional capability-oriented persistence adapters. | 🟢 **`[Implemented / Bounded]`**: Relational CRUD, eager loading, type-safe queries, migration runner, versioned field encryption, and connection-pool resilience for supported SQLx drivers/features. PostgreSQL, MySQL, MariaDB and SQLite have distinct executable matrix contracts, while MariaDB intentionally shares SQLx's MySQL protocol/backend.<br/>🟢 **`[Implemented / Bounded]`**: `#[derive(Orm)] #[orm(backend = "turso")]` supplies typed CRUD, equality filters, ordering, pagination/counts and generated/app-assigned keys through a process-wide `TursoOrm`. Its migrations are ordered, checksummed, drift-detecting and reversible. The blank/API CLI profile generates, compiles, migrates, reports status and rolls back locally, while the same typed contract passes against the official remote libSQL server. Unsupported SQLx-specific model behaviors fail during macro expansion rather than being ignored. Other SQLx-specific blueprints, ORM relations/hooks, schema auto-diff, seed generation and transparent embedded-replica synchronization are not part of this bounded Turso profile.<br/>🟢 **`[Implemented / Bounded]`**: The optional persistence boundary supplies portable document CRUD for MongoDB and SurrealDB, parameterized OLAP queries through in-process DuckDB, explicit parameterized Turso/libSQL SQL/transactions, and bounded read-only ISO GQL through SurrealDB. These capability APIs do not claim shared semantics or cross-store transactions. External adapters select deterministic offline behavior for empty or `mock_*` credentials where documented; SurrealDB uses its HTTP protocol rather than embedding the BSL-licensed SDK.<br/>🟢 **`[Implemented / Feature-gated]`**: `scout-http` provides bounded Meilisearch, Elasticsearch and Algolia indexing/search adapters plus deterministic mocks. Meilisearch has a digest-pinned live lifecycle; Elasticsearch/Algolia have protocol fixtures, not hosted-provider certification. Generated projections are process-local post-commit effects unless the application explicitly composes the transactional outbox.<br/>🟢 **`[Implemented / Feature-gated]`**: `pgvector` with `strict-postgres` supplies typed SQL vector helpers. `qdrant` supplies a separate bounded dense-vector collection/upsert/delete/cosine-query contract, while `redis` supplies namespaced Hash, Set and Sorted Set operations. All three have digest-pinned live lifecycles; RAG orchestration, authorization, production ANN tuning and Redis cluster/failover remain application/deployment boundaries. |
| **`rullst-auth`** | Argon2id password hashing, encrypted cookie sessions (AES-256-GCM), opt-in application JWTs, Passkey ceremony foundations, RBAC context guards. | 🟢 **`[Implemented / Bounded]`**: Non-blocking `spawn_blocking` Argon2id hashing, versioned expiring AES-256-GCM sessions, fail-closed `RequireRoleLayer`, compile-validated `#[rullst::require_role]`, named `Policy<User, Resource>` decisions, and a feature-gated application JWT policy with required versioned claims, bounded TTL/scopes, strong HS256 keys, `kid` rotation and a revocation-store contract that rejects process-local state in production mode. Authentication, role persistence and resource/tenant lookup remain application boundaries.<br/>🟠 **`[Partial]`**: No built-in shared durable JWT revocation/device adapter exists. Passkey registration/assertion validates the documented ES256/`none`-attestation scope, but normative WebAuthn conformance or adoption of an audited full server library remains required before a general stable claim. |
| **`rullst-security`** | Explicit extended defense-in-depth layers: bounded RASP, authenticated Vault, Login Jail, Secure Headers, rate limiting, DLP and security telemetry. | 🟢 **`[Implemented / Bounded]`**: AES-256-GCM envelopes with rotation/AAD, bounded URI/header/body RASP heuristics, local abuse controls, CSWSH origin guard, OS-random TOTP with SVG enrollment QR, strict JSON transport inspection plus an explicitly mounted reusable JSON Schema 2020-12/OpenAPI 3.1-component policy, explicit log redaction, file-backed SRI hashes, and a versioned/bounded `LiveSecurityEvent` v1 dashboard envelope. Schema construction caps bytes/nodes/depth, accepts only local references, disables network/filesystem resolution and uses linear-time regexes; auth/ownership/domain rules and query/header/form validation remain application contracts. A deterministic Sentinel classifies three caller-supplied aggregate patterns and can issue HMAC-authenticated, subject-bound, expiring, one-shot process-local proof-of-work challenges; it is not AI attribution, automatic blocking or distributed replay protection. The CLI emits bounded fail-closed evidence and a CycloneDX 1.5 Cargo SBOM; it does not certify the application.<br/>🟢 **`[Implemented / Feature-gated]`**: `redis-rate-limit` provides namespaced atomic Redis fixed-window counters, hashes client keys and exposes an explicit process-local offline mode that production can reject with `require_distributed()`.<br/>🟠 **`[Partial]`**: Recovery-code consumption must be persisted transactionally by the application. Real Redis cross-instance/eviction/failover evidence is still required. CSP nonce composition is shared, but Core and Security are not yet one canonical Server stack; WebSocket CSRF tickets/frame crypto and durable SIEM delivery are not implemented. |
| **`rullst-ai`** | Multi-provider LLM client (Gemini, OpenAI, Claude, DeepSeek, Ollama), prompt injection defenses, PII masking, bounded tenant-aware RAG, guarded local tools, and conversational memory. | 🟢 **`[Implemented / Bounded]`**: Guarded `AiClient`, heuristic prompt filter, PII masking, machine-readable provider capabilities, configurable bounded live-request deadlines, a versioned deterministic injection/jailbreak/PII regression corpus, strict URL/resolved-IP/redirect/resource policy plus an opt-in deny-by-default HTTPS fetcher with exact-host allowlist, DNS pinning, proxy bypass, peer verification and streaming limits, and local tool dispatch requiring allowlist, principal authorization, closed bounded JSON, call budget, audit sink, and payload-bound approval for destructive/financial calls.<br/>🟢 **`[Implemented / Bounded]`**: `RagPipeline::answer` composes guarded embedding, a static-dispatch application retriever, Unicode-safe per-document/total context budgets, guarded generation, source metadata, and required secret-minimized terminal audit under a trusted `TenantContext`. It rejects differently tagged, unsafe, over-returned or empty context. A bounded tenant-partitioned process-local cosine retriever supplies the offline contract.<br/>🟢 **`[Implemented / Feature-gated]`**: `StatefulChat<M>` loads bounded tenant/conversation history, performs guarded generation and atomically appends one user/assistant exchange through a static `ChatMemory`. The bounded in-memory store is always available; `sql-memory` supplies fixed-schema SQLite/PostgreSQL/MySQL/MariaDB storage with an even monotonic revision and transactional compare-and-swap, so stale cross-process writers fail instead of silently reordering. Raw message encryption/retention, authenticated ownership within a tenant, provider audit, backups and conflict UX remain application contracts; the CLI scaffold remains the Turso/custom-model path.<br/>🟠 **`[Partial]`**: The egress fetcher is not automatically mounted around provider transports or arbitrary application clients; live-origin redirect/stream contracts, live-model/adaptive evals, provider-native tool calling, explicit provider-neutral cancellation, automatic retries, durable production tool/RAG auditing, approver authentication, first-party external vector-store retrievers, authoritative datastore/domain authorization, ingestion/deletion and output policy remain application or roadmap work.<br/>🟡 **`[Offline Mock]`**: Deterministic offline chat/vision/embedding fallbacks. |
| **`rullst-capital`** | Multi-gateway billing, SaaS MRR/ARR metrics, constant-time webhook signatures, contractor payouts, and a bounded National NFS-e preparation pipeline. | 🟢 **`[Implemented / Bounded]`**: Provider-specific payment/payout adapters, pooled HTTP clients, explicit mock credentials, and signature/freshness/replay foundations for the methods documented by each adapter.<br/>🟠 **`[Partial]`**: Uniform live method coverage and durable cross-instance idempotency are incomplete; Alipay RSA2 fails closed.<br/>🟢 **`[Implemented / Feature-gated]`**: `nfse` pins the current official 1.01 production/restricted artifact profiles by SHA-256, builds a strict ordinary-service DPS subset without floating-point money, and validates extracted official XSD sources from a closed in-memory catalogue. After hash verification, the production profile receives exactly one declared compatibility normalization: .NET-style `^...$` anchors are removed from the known DPS-series pattern so the XSD-regex engine applies the authority's apparent intent instead of treating the anchors as literals. The same feature signs `infDPS/@Id` with PKCS#12 RSA-SHA256/inclusive-C14N 1.0, verifies its local XMLDSig test fixture, and constructs a bounded rustls mTLS identity/client. Its offline protocol codec emits the exact `dpsXmlGZipB64` JSON object deterministically and parses bounded synchronous 201 authorization or 400/403/500 rejection responses, binding environment, submitted DPS, access key and a cryptographically valid embedded NFS-e XMLDSig. Certificate secrets are redacted and zeroized where owned by Rullst.<br/>🟡 **`[Offline Mock]`**: Deterministic `NfseEnvironment::Mock` fixture, unmistakably not a tax authorization.<br/>🔵 **`[Roadmap / External Evidence]`**: Live transmission, full emitter-certificate/ICP-Brasil trust policy, durable idempotency/audit, restricted-environment certificate tests, independent review and SEFIN homologation. Homologation/production transmission remains fail-closed. |
| **`rullst-connect`** | Social login / OAuth2 / OIDC providers (Google, Apple, GitHub, Discord, Auth0, Cognito) with PKCE and rotating JWKS. | 🟢 **`[Implemented / Bounded]`**: OAuth2/OIDC clients with constant-time PKCE comparison, validated discovery, bounded JWKS refresh/cache policy, deterministic mock credentials and a credential-free `UniversalProfile` projection. `ConnectUser` serialization omits access/refresh tokens. The optional Axum/tower-sessions lifecycle generates a ten-minute state + PKCE challenge, adds nonce for OIDC, keeps verifier/nonce server-side, removes and immediately saves the sole active challenge before validation and rejects sequential replay/expiry/mismatch. The host still owns durable session storage and cookie/TLS/account policy; the generic session-store API is not distributed compare-and-delete, so simultaneous already-loaded callbacks require idempotent effects or a stronger application store. `ReqwestClient` also exposes explicit HTTP(S) corporate-proxy constructors: endpoint shape is bounded, URL credentials are rejected, authenticated remote proxies require HTTPS, system-proxy lookup is disabled and a local protocol fixture proves routing/auth headers.<br/>🔵 **`[Roadmap]`**: PAC/WPAD, SOCKS, proxy mTLS identity and enterprise deployment certification are not implied. Message brokers belong in a future messaging boundary rather than this OAuth-focused crate. |
| **`rullst-iot`** | `no_std` sensor telemetry models and an Ed25519-signed firmware-manifest verification gate. | 🟢 **`[Implemented / Bounded]`**: Ed25519 manifest verification with in-process anti-rollback state, target/hash/length checks, `no_std` telemetry/frame models, a credential-free local HTML snapshot renderer, and a safe telemetry-module CLI scaffold.<br/>🟠 **`[Partial]`**: GPIO state, I2C/Modbus frames, BLE GATT records, RSSI topology, power recommendation and Digital Twin JSON are data/helpers only, not hardware, network or realtime drivers.<br/>🟡 **`[Simulador Dev]`**: Deterministic MQTT/HSM/PQC fixtures require `feature = "experimental-simulators"` and never represent those production capabilities.<br/>🔵 **`[Roadmap]`**: Persistent counter/boot integration, firmware download/flashing, MQTT/CoAP transports, hardware drivers/HSM and audited ML-KEM. |
| **`rullst-mail`** | Transactional email engine with Resend, SendGrid, Postmark, optional SMTP, offline fixtures and an explicit SES-proxy boundary. | 🟢 **`[Implemented / Bounded]`**: Mandatory pre-flight pipeline, anti-CRLF validation, bounded disposable-domain/security/DLP heuristics, provider-specific transports, seven safe scaffold variants (including provenance-aware fiscal receipts and explicit D+1/D+3/D+7 dunning), and expiring purpose-bound HMAC tracking tokens. `TenantMailResolver` selects an in-process driver directly from an explicit authenticated Core `TenantContext`; invalid IDs and unavailable registry state fail closed, and tests prove two contexts do not cross-deliver. `MailError` classifies permanent/transient/rate-limit outcomes; the in-process `FailoverDriver` sends another provider only transport/HTTP 5xx/429/transient-SMTP failures, captures bounded delta `Retry-After`, fails closed on circuit-state errors and emits structured tracing without provider response bodies. `Mail::enqueue` preserves tenant and bounded due-time metadata through SQLite/Redis without early claims; the worker consumes that timestamp only after it is due. Direct Resend/SendGrid retain provider-native scheduling, while real SMTP/Postmark/Log/SES-proxy paths reject future direct delivery; offline fixtures may retain it for assertions. Fiscal mock responses remain visibly unauthorized; dunning does not infer billing state or scheduling.<br/>🟠 **`[Partial]`**: Exact execution time, exactly-once delivery and provider acceptance are not implied; attachment parity varies by transport; breaker state and alert operations are not distributed; durable encrypted tenant credentials, rotation and cross-process distribution remain application/deployment concerns; tracking payloads are authenticated but not confidential. Direct AWS SES v2 fails closed because SigV4 is not implemented; `AwsSesDriver` supports only offline fixtures or an explicit trusted bearer proxy.<br/>🟡 **`[Offline Mock]`**: Memory/Log plus empty or `mock_*` provider credentials. |
| **`rullst-studio`** | Local Developer Control Room (`http://127.0.0.1:5555`), clean route navigation, live system telemetry visualizers. | 🟢 **`[Implemented / Bounded]`**: Local control center, `RadarSnapshot` telemetry, database/migration surfaces when configured, and explicit `Unavailable` states for unconnected probes. The data browser reads/filters SQLx tables and, only after the verified debug-loopback/same-origin middleware installs an unforgeable request marker, can update primitive non-key values or delete exactly one complete-primary-key-selected row. Values are bound, request/schema/value cardinality is bounded, backend-specific types remain read-only and SQLite/PostgreSQL/MySQL/MariaDB have executable mutation contracts. This is not application tenant/RBAC, audit, rollback or shared-production administration. The supplied queue snapshot exposes only backend records; SQLite can explicitly retain 1–100,000 successful jobs with atomic pruning and purge while deleting them by default. Retained payload access/policy belongs to the host, and Redis/custom inspection remains capability-specific. Successful feature-flag toggles invalidate all warm `DbFeatureDriver` caches in the same process through a constant-size epoch. Cross-process/direct-writer invalidation remains TTL-bound unless the application distributes the signal. |
| **`rullst-nexus`** | Auto-generated Admin CMS (`/nexus`), dynamic model CRUD, AI Admin Assistant (`/nexus/chat`), SOC Threat Radar. | 🟢 **`[Implemented / Bounded]`**: `#[derive(Nexus)]` emits registered named-field metadata with inferred primitive or explicit semantic widgets; the panel provides parameterized CRUD/search/sort/pagination plus bounded selected-record delete/deactivate. Construction is fail-closed, requires an authentication policy and admin role layer, enforces server-side field policy, and escapes record/model metadata on audited paths. Deactivation requires a writable Boolean `is_active`/`active`; host identity, tenant ownership and database privileges remain application contracts. |
| **`rullst-macros`** | Procedural macros (`html!`, `rullst::model`, `rullst::runtime::main`) and compatibility helpers. | 🟢 **`[Implemented / Bounded]`**: Compile-time `html!` escaping with explicit `RawHtml`, model/runtime macros, and `trybuild` diagnostics.<br/>🟠 **`[Partial]`**: `server_function` preserves typed signatures, but browser argument transport and matching server-side RPC registration are not end-to-end. |
| **`cargo-rullst`** | Developer CLI toolkit, scaffolding generators (`make:*`), project blueprints, AST IDOR static route scanner. | 🟢 **`[Implemented / Bounded]`**: Interactive wizard, generators, heuristic IDOR scanner, CycloneDX exporter, toolchain doctor and a fail-closed Academy evidence diagnostic that explicitly does not certify a deployment. `make:chat-session` emits registered SQLx or Turso-primary models, reversible migrations and application-owned bounded chat memory; materialized contracts run persistent mock conversations on both backends and prove collision refusal. `make:billing --model` likewise emits SQLx/Turso-primary persistence plus Stripe/LemonSqueezy pricing, authenticated checkout/portal and mandatory signed-webhook code; its materialized contract compiles, migrates, persists, denies cross-owner subscription mutation before customer binding and refuses existing outputs on both backends.<br/>🟢 **`[Implemented / Bounded]`**: The LMS starter supplies bounded curriculum, school-scoped learning/assessment/publication/progress/completion, roles, leaderboard, automation/outbox/workers, localized in-app notifications and a minimized privacy-request foundation. Its SSR catalog performs limited, ORM-parameterized title/category filtering; generated auth/catalog/course/player shells consume the Core CSP nonce without remote page dependencies or inline style attributes and include keyboard landmarks, visible focus and reduced-motion handling. Privacy claims use exact leases, retry/dead-letter with a hard ten-attempt ceiling, actor/digest-bound completion and a supervised static-dispatch executor with an explicit protocol-only mock; the product must still supply the adapter that performs application-specific export/deletion/anonymization. Materialized SQLite exercises catalog escaping/nonce, privacy hard limits and the documented vertical/cross-school boundaries. Detached `--lms-modules auth`, `auth,learning` and `auth,learning,assessment` profiles remain small compiling foundations; the assessment profile grades versioned quizzes authoritatively without pulling score/leaderboard/outbox verticals. The complete starter is the default.<br/>🟠 **`[Partial]`**: Other detached combinations, profile hot reload, attachments/media, advanced/localized search, captions/transcripts, WCAG/browser evidence, distributed failover, PostgreSQL/MySQL isolation, visual authoring, exported telemetry and the separately operated Academy remain roadmap or release-engineering work. |

---

## ⚡ 4. Core API Specifications (`rullst-core`)

`rullst-core` provides the runtime kernel. Database and queue drivers are modular and feature-gated.

### 4.1. Server & Routing (`rullst::routing`)
* **Routing Macro:** Central declarative routing declared via the `routes!` macro wrapping Axum routing handlers:
  ```rust
  let router = routes![
      get("/" => home),
      get("/posts" => posts_controller::index),
      post("/posts" => posts_controller::store),
      get("/posts/:id" => posts_controller::show),
  ];
  ```
* **Server Lifecycle & Graceful Shutdown:**
  ```rust
  Server::new(router)
      .with_graceful_shutdown()
      .run(3000)
      .await?;
  ```

### 4.2. Server-Side Rendering (`rullst::macros`)
* **Macro:** `html!` expands supported HTML trees into ordinary Rust `String`
  construction at compile time.
* **XSS Protection:** Dynamic display values in the supported `{expr}` syntax
  are HTML-escaped by the generated code.
* **Raw Unescaped HTML:** Explicitly bypassed using the wrapper `rullst::html::RawHtml(String)`.
* **Example:**
  ```rust
  let username = "<script>alert('xss')</script>";
  let rendered = html! {
      <div class="user-badge">
          <span>"User: "{username}</span>
      </div>
  };
  // Automatically escapes to: &lt;script&gt;alert('xss')&lt;/script&gt;
  ```

### 4.3. Durable Queue Timing and Completion History
* SQLite and Redis persist `dispatch_at` for at most 366 days and never claim a
  job before its stored millisecond due time. Execution remains poll-dependent
  and at-least-once.
* Successful SQLite jobs are deleted by default. The explicit
  `Queue::sqlite_with_completed_history` constructor validates a 1–100,000 row
  limit, changes a processing row to `completed`, and prunes excess history in
  the same transaction.
* `Queue::purge_completed_history` removes those opt-in retained successes.
  Rows contain the original payload, so Studio access, data minimization and
  retention policy remain host responsibilities. Redis/custom drivers expose
  inspection or history only when their capability implements it.

---

## 🗄️ 5. Active Record ORM & Schema Engine (`rullst-orm`)

### 5.1. Model Definition & CRUD
```rust
#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "users")]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    #[orm(encrypted)]
    pub secret_token: Option<String>,
}

// Queries
let all_users: Vec<User> = User::all().await?;
let user: Option<User> = User::find(1).await?;

// Mutations
let mut new_user = User { id: 0, name: "Alice".into(), email: "alice@example.com".into(), secret_token: None };
new_user.save().await?; // Auto-executes parameterized INSERT or UPDATE
new_user.delete().await?;
```

### 5.2. Parameterized Queries & Privacy
* Values accepted by non-raw query APIs use SQLx parameterization. Structural
  identifiers use the bounded ASCII grammar. pgvector helpers bind canonical
  vector/distance strings after rejecting empty/non-finite vectors and invalid
  distances; they do not interpolate those runtime values. Methods explicitly
  suffixed/named `raw` remain caller-owned escape hatches rather than an
  injection-safety claim.
* Generated builders assemble bindings by emitted clause position (CTE, JOIN,
  WHERE/HAVING, ORDER BY), not by the order in which fluent methods were
  called. Nested typed subqueries export that ordered binding sequence.
* Generated magic filters bind supported primitive fields to their Rust type at
  compile time (`String`, `i32`, `f64`, and `bool`), and generated column enums
  make unknown columns unrepresentable on typed paths. String-column builders,
  custom `RullstValue` conversions and raw SQL are explicit runtime-checked or
  caller-owned alternatives, not compile-time schema verification.
* `String` and `Option<String>` fields annotated with `#[orm(encrypted)]` are encrypted before generated ORM writes and decrypted after generated model reads using AES-256-GCM. Randomized ciphertext cannot be filtered, ordered, grouped, or explicitly selected by generated query-builder methods; use a separately reviewed blind index when equality lookup is required. Raw SQL remains an explicit, non-transparent escape hatch.

### 5.3. Generated Relationship Contract

* SQLx models may declare `morph_many`, `morph_one`, and one or more explicit
  typed `morph_to` targets. A polymorphic relation requires
  `morph_name = "..."` (`name` remains a legacy alias).
* `morph_to` fails macro expansion unless the source has a persisted bindable
  `<morph_name>_id` field and a persisted `String` discriminator named
  `<morph_name>_type`. `foreign_key` may override the ID field and
  `related_key` may override the target key.
* The discriminator stores the Rust target model name. Lazy loading returns
  `None` for a different target; eager loading batches each declared target and
  never guesses an undeclared runtime type. Target models used in eager inverse
  loading must implement `Clone`.

### 5.4. Tenant Scope Contract

* A SQLx model declaring `#[orm(tenant_column = "tenant_id")]` must have a
  persisted `String`, `i32`, `f64`, or `bool` tenant field. The derive rejects a
  missing or unsupported field type.
* Generated queries fail closed when called outside `with_tenant(...)` and bind
  the active tenant inside the scope. Generated full/partial updates and
  instance delete/restore paths reject a model from another tenant.
* `Model::unscoped()` is the explicit global escape hatch. Deciding who may use
  it, deriving tenant identity from authenticated state, and database-level RLS
  remain host responsibilities.
* SQLx builders keep offset-based `chunk(...)` for compatibility and expose
  fallible `chunk_by_id(...)`/`chunk_by_id_with_tx(...)` for stable ascending
  keyset traversal over the generated `i32` primary key. This prevents deletes
  of processed rows from shifting later rows behind an offset; it is not a
  database-server cursor or a universal cross-shard snapshot.
* A model delete with marked `cascade_soft_delete` has-one/has-many relations
  runs parent and direct-child mutations in one transaction. An existing
  explicit or task-scoped transaction is reused; otherwise `delete()` opens,
  commits, or rolls back its own transaction. Recursive descendant/cycle
  traversal remains a separate contract.
* Generated `#[orm(auditable)]` instance `save()`/`delete()` operations write
  their bounded audit entry through the same explicit, implicit, or task-scoped
  transaction as the model mutation. Audit write errors fail the mutation and
  roll its savepoint back; direct `log_audit` calls also honor a task-scoped
  transaction. Bulk builders do not synthesize per-row history. Actor/tenant
  identity and revision restore remain separate application or roadmap
  contracts.

### 5.5. Process-Local Post-Commit Contract

* `Orm::transaction` and direct generated model `save()`/`delete()` operations
  own a post-commit callback scope. `after_commit` callbacks registered within
  it run only after SQLx confirms commit and are discarded on rollback. When no
  managed transaction is active, `after_commit` executes immediately for an
  already committed/autocommit operation.
* Generated observers retain synchronous lifecycle callbacks such as
  `creating`, `created`, and `saved` for mutation validation. The separate
  `committed(ModelCommittedEvent)` callback receives an owned, hidden-field-
  aware snapshot after the managed commit. Generated Redis invalidation/pub-sub
  and Scout projections use this same post-commit boundary.
* Every queued callback is attempted. A failure is returned as `PostCommit`,
  whose contract explicitly means the database mutation is already durable.
  Applications must not retry the database mutation blindly from this error.
* A caller-owned raw SQLx transaction passed to `save_with_tx` or
  `delete_with_tx` does not expose its later commit/rollback decision to the
  ORM. Use `Orm::transaction` for the strict process-local boundary.
* These callbacks do not survive process failure and provide no retry,
  idempotency or cross-node delivery. Use the explicit durable outbox below
  for an irreversible or externally delivered effect; it is not enabled
  automatically by a generated observer.

### 5.6. Durable Transactional Outbox Contract

* `Outbox::enqueue` accepts only a currently managed `Orm::transaction` and
  writes `rullst_outbox` through that same transaction. A domain rollback also
  removes the event. `enqueue_with_tx` provides the equivalent explicit path
  for a caller-owned SQLx transaction. No implicit independent commit is
  permitted.
* `(stream, event_key)` is the database uniqueness boundary. Replaying the same
  key and exact event kind/payload returns the existing `i64` identifier;
  reusing the key with different content fails closed. `stream`, event key,
  event kind and worker identifiers use a bounded ASCII grammar, and serialized
  payloads are limited to one MiB.
* PostgreSQL, MySQL/MariaDB and SQLite share the outbox state machine. A claim
  increments attempts and receives a random token plus a bounded lease. Only
  that token may acknowledge or fail the event; expiration permits another
  worker to reclaim it. Failure schedules a bounded retry or moves the event to
  `dead_letter` at the configured attempt limit, including a worker that dies
  while holding its final lease.
* Delivery is **at least once**, not exactly once. A worker may perform its
  external effect and crash before acknowledgement, so consumers must use the
  stable stream/event key as their own idempotency key. Ordering across retries
  or concurrent workers is not guaranteed.
* `Outbox::install` is an explicit setup/test convenience and never runs at
  startup. `OutboxMigration` puts the same schema under the built-in reviewed
  migration lifecycle. The ORM does not infer tenant authorization from
  `stream`, automatically serialize model observers, dispatch HTTP webhooks,
  purge delivered rows or promise cross-database transactions.

### 5.7. Generated Redis Query Cache Contract

* The optional `redis` feature enables `.remember(seconds)` for generated SQLx
  reads. `Orm::init_redis_with_namespace(url, application_namespace)` is the
  recommended initializer when a Redis database is shared; the compatibility
  `init_redis(url)` initializer uses the literal namespace `default`.
* Versioned SHA-256 cache keys bind the validated application namespace, an
  opaque digest of the active tenant scope when present, table, generated SQL,
  and typed bindings. Raw tenant identifiers are not emitted in keys.
* Generated reads always bypass Redis inside explicit and task-scoped database
  transactions, so cached state cannot replace the transaction's own view.
  `remember(0)` is invalid. Outside transactions, explicitly requesting cache
  without initializing Redis fails closed as a configuration error; transport
  failures and corrupt cached JSON fail open to the authoritative database.
* Cache writes occur only after a successful database read and retain encrypted
  model fields as ciphertext. Generated model `save()`/`delete()` operations
  invalidate the active tenant/table's versioned keys only after commit through
  a bounded Redis `SCAN` plus asynchronous `UNLINK`; rollback preserves existing entries.
  Raw SQL, bulk builders, caller-owned raw transactions and writes from other
  processes cannot be inferred. Callers must retain a defensive TTL and treat
  Redis cluster/failover and durable invalidation delivery as separate
  application contracts.

### 5.8. Polyglot Persistence Boundary

* Optional persistence adapters are disabled by default and selected with
  `mongodb`, `duckdb`, `turso`, `surrealdb`, `qdrant`, or the `polyglot` convenience
  feature. The umbrella crate exposes matching `orm-*` features.
* `DocumentRepository<T>` provides create, find, replace, delete, and
  deterministic bounded listing. Collection names, document IDs, offsets and
  limits are validated before reaching a driver.
* `MongoDbStore<T>` uses the official MongoDB Rust driver and stores the
  portable `DocumentId` as `_id`; portable models must not define `_id`.
* `DuckDbStore` serializes access to its native connection and delegates every
  database operation to `spawn_blocking`. Dynamic values use prepared
  parameters, and callers must supply a `QueryLimit` before rows are
  materialized. Application-provided SQL text remains a trusted structural
  input.
* `TursoStore` speaks the official Hrana HTTP v3 protocol directly for remote
  edge SQL. It uses positional typed parameters, conditional atomic batches,
  a 30-second request deadline, no redirects, a 16-MiB response bound, bounded
  row materialization, and ordered checksummed migrations. This avoids an
  unnecessary embedded/native SDK dependency while retaining conformance
  against the official libSQL server. Empty or `mock_*` endpoints use a
  one-connection SQLite fallback that exercises real SQL without pretending to
  be a remote replica. HTTPS/`libsql://` is required outside explicitly enabled
  loopback development.
* `SurrealDbStore<T>` uses the documented `/key`, `/sql`, and `/gql` HTTP
  endpoints with namespace/database headers, no redirects, bounded streaming
  responses, HTTPS by default, and redacted authentication configuration.
  `GraphQuery::read_only` accepts one `MATCH` query, rejects mutation tokens
  and caller-supplied limits, then appends a bounded limit.
* This boundary does not turn every backend into SQL Active Record, perform
  cross-database transactions, synchronize records between engines, or prove
  a third-party deployment. See the
  [Polyglot Persistence guide](polyglot-persistence.md).

### 5.9. Scout Search Projection Contract

* `#[orm(searchable)]` projects generated save/delete operations only after a
  managed relational commit. Search adapter failures remain visible; a failed
  query is not silently treated as an empty result, and `PostCommit` means a
  projection failed after the database mutation became durable.
* `MockSearchEngine` is deterministic and always available. The optional
  `scout-http` feature adds Meilisearch, Elasticsearch and Algolia. Empty or
  `mock_*` credentials select the mock; keyless live constructors accept only
  loopback HTTP, while remote/custom origins require HTTPS without URL
  credentials, redirects, paths, queries or fragments.
* Index names, positive IDs, object payloads, queries, response bytes and hit
  counts are bounded. Meilisearch/Algolia tasks use bounded polling;
  Elasticsearch requests use `refresh=wait_for`. Provider response bodies and
  credentials are not copied into transport errors.
* The repository proves a real Meilisearch lifecycle in a digest-pinned
  container. Elasticsearch and Algolia protocol fixtures prove the documented
  HTTP shape and bounds, not hosted service operation, version-wide
  compatibility, ranking quality or cluster failover.
* The generated hook remains process-local. Guaranteed crash recovery requires
  an application-versioned event in the transactional `Outbox` and an
  idempotent worker; the ORM cannot infer a safe event key or external retry
  policy from an arbitrary model save.

### 5.10. PostgreSQL pgvector Contract

* The optional `pgvector` feature re-exports the SQLx-compatible `Vector` type.
  The supported execution profile combines it with `strict-postgres`; other
  SQLx backends do not pretend to implement PostgreSQL vector operators.
* `where_similar`, L2, cosine and inner-product ordering validate column names,
  reject empty/non-finite vectors and invalid distances, and bind vector and
  distance values. ORDER BY bindings are assembled after WHERE bindings
  regardless of builder call order.
* A digest-pinned PostgreSQL + pgvector container installs the extension, uses
  a typed vector model and proves L2 threshold/cosine ordering queries. The
  application still owns reviewed migrations, vector dimensions, embedding
  model compatibility, HNSW/IVFFlat index selection/tuning, tenant policy,
  context budgets, citations, ingestion/deletion and RAG evaluation.

### 5.11. Qdrant and Redis Specialized Store Contract

* The optional `qdrant` feature exposes a separate `VectorRepository` rather
  than pretending Qdrant is SQL Active Record. Collection names, dimensions,
  vectors, cosine norm, point payloads, query limits and response bytes are
  bounded. The HTTP client rejects redirects and URL credentials, uses short
  connect/request deadlines, requires HTTPS outside loopback, redacts API keys,
  and never copies provider response bodies into errors.
* `QdrantConfig::new` selects a deterministic in-process fallback for empty or
  `mock_*` endpoint/API-key values. `unauthenticated_local` is an explicit
  loopback-only path for self-hosted development. The supported live API is one
  unnamed dense cosine vector per numeric point with create, single-point
  upsert/delete and bounded nearest-neighbor query; named/sparse/multivectors,
  arbitrary filters, collection tuning and distributed topology are outside it.
* The optional `redis` feature exposes `RedisDataStore` for explicitly
  namespaced Hash, Set and Sorted Set operations in addition to the generated
  query cache. Keys, fields, UTF-8 values/members, finite scores and scan/range
  materialization are bounded. Remote endpoints require `rediss://`; URL
  credentials are rejected, ACL credentials are redacted, operations have
  connection/response deadlines, and empty/`mock_*` credentials select a
  deterministic in-process fallback.
* Digest-pinned Qdrant and Redis matrices prove their respective live
  lifecycles, including Redis namespace/structure separation. They do not prove
  hosted-provider availability, backups, cluster failover, tenant
  authorization, eviction policy, ANN quality, or cross-store transactions.

### 5.12. ORM Telemetry Contract

* Generated model/query entrypoints, transaction-aware variants, raw ORM
  queries and generated streams emit `rullst.orm.query` spans with only a
  static model, validated table and bounded operation name. SQL text, bindings,
  model values, DSNs and error strings are not fields of these Rullst-owned
  spans. The explicit debug query logger remains a separate opt-in surface.
* Managed transactions emit begin and lifecycle spans. Their final outcome is
  one of the bounded commit/rollback states; transaction errors are returned to
  the caller rather than copied into telemetry. Generated stream spans are
  entered only while the stream is polled, so a tracing guard is never held
  across suspension.
* Every pool constructed through `Orm::init*` emits SQLx pool-acquire timing at
  info level and promotes acquisitions slower than 500 ms to warnings. Primary
  and replica pools share this configuration. Direct pools constructed by the
  application are outside the contract.
* These standard `tracing` spans/events are exported when the host enables the
  umbrella `telemetry` feature and initializes Core's OpenTelemetry subscriber.
  The host still owns OTLP endpoint security, filters, sampling, retention and
  collector availability. SQLx or application logging configured separately
  may have its own statement-data policy.

---

## 💳 6. Billing, Payments & Fiscal Engine (`rullst-capital`)

`rullst-capital` exposes bounded billing-provider and payout-provider adapters,
plus a bounded Brazilian digital-invoicing preparation pipeline (NFS-e
Nacional). Local cryptographic/schema validity is not tax authorization.

### 6.1. Multi-Gateway Payment Architecture
Billing adapters implement `BillingProvider`; the Wise payout adapter implements
the separate `PayoutProvider` contract. Individual billing operations may still
return `Unsupported` when a provider adapter has no reviewed implementation:
```rust
use rullst_capital::providers::stripe::StripeProvider;
use rullst_capital::providers::BillingProvider;

let provider = StripeProvider::new(api_key, webhook_secret);
let session = provider
    .create_checkout_session(customer_email, plan_id, redirect_url)
    .await?;
```

`#[derive(rullst::Billable)]` is the umbrella convenience for named structs with
an `email: String` field. It preserves generics; optional
`subscription_id: Option<String>` and `tier: Option<String>` fields expose the
corresponding helpers. An all-or-none
`grace_period_starts_at: Option<i64>`/`grace_period_ends_at: Option<i64>` pair
exposes a validated half-open window of at most 366 days. A provider-bound
`SubscriptionHandle<P>` delegates cancellation and pausing; the explicit
`subscription_with` path keeps static dispatch. These values do not infer or
persist ownership, team membership, entitlement, currency, payment methods,
usage, provider scheduling or database-backed quotas.

### 6.2. Invoice Rendering

`Invoice::generate_html` escapes every application-supplied textual field. It
renders HTML only: native PDF generation and automatic delivery after a payment
event are not part of the current contract.

### 6.3. Webhook Signature Verification
* The Axum and opt-in Actix middleware adapters call one canonical bounded
  verifier before dispatch. Built-in provider adapters use provider-appropriate
  cryptographic verification; equality checks for derived signatures are
  constant-time where applicable.
* Timestamped protocols enforce a bounded freshness window. Applications still
  need durable event-id idempotency across processes.

### 6.4. NFS-e Nacional Specification (`FiscalEngine`)
* 🟢 **`[Implemented / Bounded]` DPS 1.01 Builder:** `NfseDpsV101` models an ordinary domestic-service subset, validates CPF/CNPJ/IBGE/identifier/text limits, keeps BRL values in integer cents and ISS rates in basis points, and emits an unsigned DPS in the official namespace. The legacy floating-point preview remains compatibility-only.
* 🟢 **`[Implemented / Bounded]` Pinned Schema Validation:** Production profile `v1.01-20260209` and restricted profile `v1.01-20260727` carry immutable archive/file SHA-256 values. `NfseDpsSchemaValidator` reads only the expected bounded files and resolves imports from an in-memory catalogue; it never downloads schemas or follows instance hints.
* 🟢 **`[Implemented / Bounded]` Local XMLDSig and mTLS Preparation:** `sign_dps_xml` parses a protected PKCS#12 A1 container, rejects malformed/duplicate/already-signed envelopes and emits an enveloped inclusive-C14N 1.0 RSA-SHA256 signature over the unique `infDPS/@Id`. The matching certificate chain is embedded and tested with independent local verification. The same container can construct a rustls mTLS identity/client with HTTPS-only, no redirects, and bounded timeouts.
* 🟢 **`[Implemented / Bounded]` Offline SEFIN Issuance Codec:** `NfseIssueRequest` accepts only one structurally bound and cryptographically valid embedded DPS XMLDSig, emits deterministic GZip/Base64 inside the exact `dpsXmlGZipB64` JSON object, and parses at most four MiB. HTTP 201 can become `Authorized` only when environment, submitted DPS ID, 50-digit access key, `infNFSe/@Id` and the embedded NFS-e XMLDSig agree; HTTP 400/403/500 become a separate bounded `Rejected` variant. Unknown fields, malformed JSON/XML/Base64/GZip, duplicate/confused IDs, invalid signatures and decompression amplification fail closed. Embedded-signature validity does not establish ICP-Brasil trust or emitter ownership.
* 🟡 **`[Simulado]` Offline Mock Environment:** `NfseEnvironment::Mock` produces deterministic test fixtures for local sandboxing.
* 🔵 **`[Roadmap / External Evidence]` Official SEFIN Homologation & Production:** `Homologation` and `Production` validate credentials and then return `FiscalError::Unsupported` without network I/O. Enabling transmission requires emitter-certificate/ICP-Brasil lifecycle checks, durable idempotency/audit, retained protocol fixtures, real restricted-environment tests with an authorized contributor and municipality, independent review, and successful official homologation.

---

## 🛡️ 7. Enterprise Security, RASP & Vault (`rullst-security`)

### 7.1. Rullst Vault (Authenticated Field Encryption)
* **Algorithm:** AES-256-GCM with authenticated 96-bit random nonces and 128-bit authentication tags.
* **Envelope Format:** `RULLST:v2:<key_id>:<base64_nonce>:<base64_ciphertext_and_tag>`.
* **Key Rotation:** Built-in keyring support (`decrypt_with_keyring`) can read
  prior keys while new writes use the active key. Deployment coordination,
  re-encryption, key custody and retirement remain operator responsibilities.
* **ORM Configuration:** `RULLST_ENCRYPTION_KEY`, `RULLST_ENCRYPTION_KEY_ID`, and `RULLST_ENCRYPTION_KEYRING` select the current and still-readable prior keys. Rullst does not provide key custody or automatic retirement.

### 7.2. Runtime Application Self-Protection (RASP)
* **Bounded Heuristic Inspector:** ASCII case-insensitive signature matching covers selected SQL injection, traversal, SSRF, shell/JNDI patterns across URI, non-secret headers, and supported bounded textual/JSON bodies. Percent decoding and body/JSON inspection allocate; this control does not replace typed parsing, SQL binds, validation, authorization, or SSRF allowlists.
* **Login Guard Tarpit:** `record_login_failure` returns progressive delay
  decisions and `record_login_failure_and_wait` applies them asynchronously;
  both share bounded, temporary in-memory jails keyed by a hashed identity.

### 7.3. MFA and Security Evidence Boundaries
* **TOTP enrollment:** Secrets contain 160 bits derived from the OS RNG,
  verification accepts exactly six ASCII digits with constant-time comparison,
  and enrollment can emit an `otpauth://` URI or bounded SVG QR. Secret custody,
  recovery workflow and durable rate limiting belong to the application.
* **Security CLI:** CycloneDX generation, MSRV/tool diagnostics, unsafe/IDOR
  source heuristics, network observations and compliance evidence are bounded
  checks. They do not certify a deployment, prove absence of vulnerabilities or
  replace provider/CI evidence tied to an immutable SHA.

### 7.4. Bounded JSON Schema Enforcement
* `JsonSchemaPolicy::from_schema` compiles an application-supplied JSON Schema
  2020-12 document once; `from_openapi_component` selects one explicit
  `components.schemas` entry from OpenAPI 3.1. OpenAPI 3.0 is rejected because
  it is not the same schema dialect.
* Construction caps serialized bytes, node count and depth, rejects non-local
  `$ref`/`$dynamicRef`, disables network/filesystem retrieval and selects the
  linear-time regex engine. The route-scoped Axum middleware first enforces the
  existing exact media-type, syntax, duplicate-key, payload-size and depth
  boundary, then returns `422` for schema mismatch without echoing values.
* The policy validates JSON bodies only. Authentication, authorization,
  ownership, business invariants and query/header/form parameters remain
  separate application boundaries.

### 7.5. Deterministic Threat Sentinel and Proof of Work
* `ThreatClassifier` assesses a bounded aggregate window supplied by the host
  against transparent thresholds for credential stuffing, API scraping and
  distributed automation. It does not collect traffic, infer identity, use a
  model or attribute a botnet.
* `ProofOfWorkGate` issues OS-random, HMAC-authenticated challenges bound to one
  canonical application subject. Tokens have bounded difficulty, TTL and
  cardinality; successful verification atomically consumes local state so only
  one concurrent verifier succeeds in the process.
* Classification is evidence, not authorization. The host chooses whether and
  where to challenge, provides an accessible fallback, rate-limits issuance and
  owns trusted proxy/device policy. Replay state is process-local; distributed
  enforcement, durable telemetry and cross-process one-shot consumption require
  an application adapter.

---

## 📡 8. IoT, Firmware Security & Protocol Frames (`rullst-iot`)

### 8.1. Ed25519 OTA Firmware Gate
* **Firmware Verification:** Strict Ed25519 signature validation over a cryptographic manifest `[target, version, rollback_counter, firmware_len, firmware_sha256]`.
* **Anti-Rollback Protection:** Rejects any firmware update proposing a monotonic rollback counter lower than or equal to the committed hardware state.
* **Commit Invariant:** Partition swapping is blocked until full cryptographic verification succeeds.

### 8.2. Embedded Sensor Frames (`#![no_std]`)
* `rullst-iot` core models compile under bare-metal `#![no_std]` targets (STM32, ESP32-C3, Cortex-M).
* `cargo rullst make:iot <DeviceName>` generates and registers a local telemetry
  module, enables the umbrella `iot` feature and refuses unsafe names or
  collisions. It does not install firmware, a HAL, MQTT or CoAP.
* `IotDashboard` renders an escaped HTML snapshot. It does not infer online
  state or provide a live device connection.
* 🔵 **`[Roadmap]` MQTT 5.0 Transport:** High-performance async MQTT 5.0 client integrating `rumqttc` with QoS 0/1/2 and automatic topic subscriptions.

---

## 🤖 9. AI Agent & LLM Orchestration (`rullst-ai`)

### 9.1. Guarded AI Client
* Provider-agnostic interface for **Google Gemini, OpenAI, Anthropic Claude, DeepSeek, and Ollama**.
* **Prompt Injection Firewall:** Real-time token heuristics intercepting prompt exfiltration, instruction overrides (`DAN mode`), and delimiter injection attacks.
* **Automated PII Masking:** Scrubs sensitive data (CPF/CNPJ, credit cards, emails) prior to outbound LLM dispatch.

### 9.2. Bounded Tenant-Aware RAG
* `RagPipeline::answer` requires a trusted Core `TenantContext` and composes
  guarded embedding, a static-dispatch application `RagRetriever`, bounded
  context selection, guarded generation, source metadata, and one required
  terminal `RagAuditSink` event.
* Retrieved documents carry the trusted tenant tag. The pipeline rejects
  mismatches, over-return, injection heuristics, empty context, non-finite
  embeddings, and unavailable mandatory audit evidence rather than silently
  generating an ungrounded response.
* Context limits count Unicode scalar values per document and in total. The
  audit event omits raw question, context, embeddings, provider bodies, and
  answer; its SHA-256 query digest is correlation metadata, not encryption.
* `InMemoryRagRetriever` and `InMemoryRagAuditTrail` are bounded process-local
  development/test implementations. Production hosts own authoritative
  tenant/ownership predicates, durable/external vector adapters, ingestion and
  deletion, model/vector compatibility, output policy, durable audit, tuning,
  evaluation, and recovery.

### 9.3. Bounded Conversational Memory
* `StatefulChat<M>` uses static dispatch over `ChatMemory`, requires a trusted
  `TenantContext` and validated `ConversationId`, loads only the configured even
  number of recent messages, applies the guarded `AiClient`, and persists the
  user/assistant exchange only after generation succeeds.
* `InMemoryChatMemory` is deterministic, tenant-partitioned, cardinality-bound,
  and intended for tests/local use. The opt-in `sql-memory` adapter supports
  SQLite, PostgreSQL, MySQL, and MariaDB through a dedicated SQLx Any pool.
* The SQL adapter advances an even conversation revision and inserts both
  messages in the same transaction. A compare-and-swap predicate rejects stale
  cross-process writers; Rullst deliberately does not retry the provider call.
  History reads bind the tenant/conversation and never include rows newer than
  the revision observed by that read.
* Message text is not encrypted by this adapter. Authenticated conversation
  ownership within a tenant, retention/erasure, provider audit, backups,
  migration governance, and user-facing conflict retry remain host policy. The
  generated Turso/custom-model scaffold is a separate application-owned path.

---

## 📊 10. Control Center & Admin Interfaces (`rullst-studio` & `rullst-nexus`)

### 10.1. Rullst Studio (`http://127.0.0.1:5555`)
* Local-first developer dashboard with a server-rendered dark interface; browser
  assets and final page policy remain deployment concerns.
* Process observations sourced from `RadarSnapshot::collect()` and explicitly
  supplied local collectors; unsupported values remain unavailable.
* Generated applications start the standalone Studio only in debug builds and
  bind it to loopback. Its local capability verifies the direct loopback peer,
  accepts only a local `Host` authority, requires same-origin `Origin` on unsafe
  methods, and rejects missing origins on mutations. This is a local
  DNS-rebinding/CSRF boundary, not production authentication.
* Queue, revenue, security and telemetry pages report only values supplied by
  their configured process-local source. Unsupported driver operations and
  disconnected integrations remain errors or `Unavailable`. The standalone
  migration surface provides CLI guidance and returns `501` from legacy
  mutation handlers because no migration/seeder registry is installed.
* The database browser accepts a deliberately narrow ASCII SQL-identifier
  boundary. Reads are bounded; writes require the crate-private proof inserted
  by the verified local middleware, database-inspected table/column/complete-PK
  metadata, a 64 KiB request limit, primitive typed binds and exactly one
  affected row. Primary keys/backend-specific values are read-only, while
  delete requires `DELETE <table>`. SQLite, PostgreSQL, MySQL and MariaDB run
  separate mutation contracts. This is not application authorization, tenant
  scoping, audit, rollback or shared-production administration. The ER diagram
  inspects the same relational backends with bound lookup values and strict
  normalized Mermaid identifiers. Swagger requires an application-supplied
  `OpenApi`.
* Request SSE records method, URI, status, and latency without bodies or headers.
  Environment values are redacted by default and the typed config projection
  never renders connection URLs, filesystem paths, cookies, tokens, or
  credentials. A successful Studio database-flag mutation invalidates warm
  `DbFeatureDriver` caches in the same process; direct writers and other
  processes remain subject to TTL unless the host distributes invalidation.
  SQLite removes completed jobs by default. An explicit 1–100,000-row history
  policy retains and atomically prunes real completion records for Studio, with
  a separate purge; retained payload access and lifecycle belong to the host.
  Redis/custom queue inspection remains capability-specific.
* Exposing Studio beyond the developer machine requires an explicit
  authenticated network boundary owned by the application; no environment
  variable silently converts the local server into a production admin surface.

### 10.2. Rullst Nexus (`/nexus`)
* Auto-generated CMS with dynamic CRUD operations and AI Admin Assistant.
* **Security Default:** Fail-closed by design; requires explicit authentication middleware and RBAC role validation (`admin`) on all mutating endpoints.
* Generated applications may use `NexusAuthPolicy::local_development_or_basic_from_env()`: debug builds accept only a peer address verified as loopback through `ConnectInfo`, while release builds require validated Basic Auth credentials from the environment. Missing peer metadata is denied, and an environment mode flag cannot enable unauthenticated release access.

---

## 🛡️ 11. Architectural Guidelines for Backward Compatibility

1. **`#[non_exhaustive]` on Public Structs:** All configuration structs and enums must use `#[non_exhaustive]` to ensure minor versions can add fields without breaking downstream code.
2. **Deprecation Policy (`#[deprecated]`):** Public APIs will never be removed without at least one minor release cycle marked with `#[deprecated]`.
3. **Ergonomic String Constructors:** Public constructors accept `impl Into<String>` to support both `&str` literals and owned `String` parameters without boilerplate.
4. **Zero-Panic Invariant:** Production paths must never call `panic!()`, `unwrap()`, or `expect()`; domain errors must return typed `Result<T, AppError>`.

---

## 🔄 12. Assisted Framework Upgrade Contract

`cargo rullst upgrade` is the canonical application-upgrade boundary. It is an
assistant, not a claim that compilation proves production compatibility.

* 🟢 **`[Implemented / Bounded]` Planning:** `--dry-run` enumerates only Cargo
  workspace manifests, preserves TOML comments/order, understands normal,
  inline-table, workspace, target-specific and renamed Rullst dependencies, and
  reports path/git dependencies that have no version instead of guessing.
  `--dry-run --json` emits the versioned `rullst.upgrade-plan.v1` envelope.
* 🟢 **`[Implemented / Bounded]` Versioned Rules:** source findings are selected
  from a versioned rule catalog using detected source majors and the exact
  target major. Every future major release must extend that catalog, migration
  documentation, negative tests and process-level fixtures for its supported
  upgrade paths.
* 🟢 **`[Implemented / Bounded]` Transaction:** the default target is the exact
  installed `cargo-rullst` version; `--to` accepts only the same major train as
  that CLI. Before writes, the command snapshots workspace manifests, the root
  lockfile and Rust sources under `target/rullst-upgrades`. It applies only
  dependency edits and compiler-provided `cargo fix` changes, then requires
  `cargo check --workspace --all-targets` to pass. A failed gate restores the
  snapshot by default; `--keep-on-failure` is explicit, and `--restore` can
  recover a persisted, path-validated snapshot after an interruption.
* 🟠 **`[Manual Application Boundary]`** the command never installs a CLI,
  changes secrets, executes database migrations, invents authorization or
  tenant policy, exposes Nexus/Studio, validates providers, or declares an
  application production-ready. Database restore/migration/rollback, the full
  test suite, authorization negatives and deployment smoke tests remain
  mandatory human-owned gates.

---

## 📱 13. Omni Packaging Contract

`cargo rullst make:omni` generates an application-owned Tauri packaging shell;
it is not a native-runtime abstraction or store-publication service.

The canonical product is the Rullst web application. Server-side domain rules,
authentication, authorization, persistence, realtime policy and security
controls remain authoritative and must work without trusting the platform
shell. Omni is **web-first, platform-enhanced**: it may add narrowly scoped
native capabilities, but it must not fork the business/security model or move
authoritative secrets into JavaScript or an untrusted client.

* 🟢 **`[Implemented / Bounded]` Deterministic Scaffold:** `--platform` accepts
  desktop, Android and iOS selections without a prompt. Mobile selections
  require `--backend-url`; HTTPS is required except for explicitly bounded
  loopback/emulator development hosts, and embedded credentials are rejected.
  Product name and application version default to validated Cargo package
  metadata. `--product-name`, `--app-version` and `--identifier` provide
  deterministic overrides. Android/iOS require an application-owned lowercase
  reverse-DNS identifier and reject framework/reserved example placeholders;
  desktop-only development may use a clearly documented `com.example` value.
* 🟢 **`[Implemented / Bounded]` Reproducible Tooling:** the generated manifest
  pins the Tauri CLI and Rust dependencies, emits a restrictive local CSP and
  real source-derived platform icons, and treats npm, icon generation or
  explicitly requested mobile initialization failures as command failures.
  Explicit iOS initialization requires macOS/Xcode.
* 🟢 **`[Implemented / Bounded]` Remote-content Boundary:** the generated local
  bootstrap exposes no Tauri IPC API to the remote application. A native
  navigation callback permits only Tauri's packaged origin and the exact
  scheme/host/effective-port tuple of the configured backend; cross-origin
  links and OAuth must use a separately reviewed system-browser/deep-link flow.
  The bootstrap provides an accessible initial offline/retry state, but this is
  not offline application data or synchronization.
* 🟢 **`[Implemented / Bounded]` Desktop Lifecycle:** the one-command local
  `http://localhost:3000` development profile owns its child process, refuses a
  pre-existing port rather than attaching to an unknown process, stops on early
  child exit or timeout, and terminates only the child it spawned. HTTPS and
  other configured origins are treated as externally operated backends.
* 🟠 **`[Authored / Hosted Evidence Pending]` Compile Evidence:** path-aware
  workflows create disposable hosts and compile fresh desktop shells on Linux,
  macOS and Windows, an Android debug APK, and an iOS simulator target. A
  workflow file is not evidence until its jobs pass on the referenced commit.
* 🟠 **`[Application / Platform Boundary]`** bundle identity, signing and
  provisioning, privacy manifest and usage declarations, native capabilities,
  production endpoint/auth policy, physical-device testing, TestFlight,
  Play testing, metadata and store review belong to the generated application.
  Offline sync, push, biometrics, OS secure storage, deep links and signed
  updates are not implied by the web shell and require opt-in capability scopes
  plus platform tests. Simulator/APK compilation must never be described as
  store acceptance or universal iPhone/Android
  compatibility.
