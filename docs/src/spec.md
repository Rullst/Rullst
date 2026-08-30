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
| **`rullst-core`** | Kernel HTTP runtime, `routes!`, Server bootstrap, HTML engine, async task queues, WebSockets, circular telemetry buffers, storage facade, and the default baseline CSRF/WAF/header/PII stack. | 🟢 **`[Implemented / Bounded]`**: Routing, server lifecycle, `html!` engine, graceful shutdown, backpressure guard, in-memory queues, and local storage with path-traversal protection. `TenantStorage`, `TenantCache`, `TenantRealtime` and `TenantPresence` bind those facades to a validated `TenantContext`, apply immutable tenant namespaces and prove same-name local non-interference; the realtime wrappers also bound channel/event/identity names and payload size. Remote bucket policy, distributed transport/liveness and application room authorization remain deployment/application work.<br/>🟢 **`[Implemented / Bounded]`**: The in-memory upload admission contract enforces a hard size/allowlist boundary, canonical tenant/name, recognized signature versus MIME/extension, active-text denial, randomized tenant quarantine keys, SHA-256 binding and fail-closed scanner release. It is not multipart streaming, a deep parser, remote persistence or a production malware engine.<br/>🟢 **`[Implemented / Bounded]`**: Validated environment precedence is `RULLST_ENV`, legacy `APP_ENV`, then `[app].env`; invalid values fail instead of silently enabling development.<br/>🟢 **`[Implemented / Bounded]`**: `apply_security_baseline` and `Server` compose configured CSP nonce headers, exact-origin CORS with explicit credential opt-in, bounded WAF, double-submit CSRF and optional PII masking in one tested order, with the per-application config installed outside every middleware. Browser/proxy/TLS deployment evidence and application-owned session/auth/tenant/authorization remain separate. A fail-closed typed Academy boundary-assessment contract records those application observations without certifying them, and the extended `rullst-security` stack is still composed explicitly.<br/>🔵 **`[Roadmap]`**: Native S3/R2 direct cloud drivers. |
| **`rullst-orm`** | Active Record & Repository patterns, parameterized SQLx connection pool (PostgreSQL, MySQL/MariaDB, SQLite), typed Turso/libSQL primary profile, schema migrations, AES-256-GCM privacy, and optional capability-oriented persistence adapters. | 🟢 **`[Implemented / Bounded]`**: Relational CRUD, eager loading, type-safe queries, migration runner, versioned field encryption, and connection-pool resilience for supported SQLx drivers/features. PostgreSQL, MySQL, MariaDB and SQLite have distinct executable matrix contracts, while MariaDB intentionally shares SQLx's MySQL protocol/backend.<br/>🟢 **`[Implemented / Bounded]`**: `#[derive(Orm)] #[orm(backend = "turso")]` supplies typed CRUD, equality filters, ordering, pagination/counts and generated/app-assigned keys through a process-wide `TursoOrm`. Its migrations are ordered, checksummed, drift-detecting and reversible. The blank/API CLI profile generates, compiles, migrates, reports status and rolls back locally, while the same typed contract passes against the official remote libSQL server. Unsupported SQLx-specific model behaviors fail during macro expansion rather than being ignored. Other SQLx-specific blueprints, ORM relations/hooks, schema auto-diff, seed generation and transparent embedded-replica synchronization are not part of this bounded Turso profile.<br/>🟢 **`[Implemented / Bounded]`**: The optional persistence boundary supplies portable document CRUD for MongoDB and SurrealDB, parameterized OLAP queries through in-process DuckDB, explicit parameterized Turso/libSQL SQL/transactions, and bounded read-only ISO GQL through SurrealDB. These capability APIs do not claim shared semantics or cross-store transactions. External adapters select deterministic offline behavior for empty or `mock_*` credentials where documented; SurrealDB uses its HTTP protocol rather than embedding the BSL-licensed SDK. |
| **`rullst-auth`** | Argon2id password hashing, encrypted cookie sessions (AES-256-GCM), opt-in application JWTs, Passkey ceremony foundations, RBAC context guards. | 🟢 **`[Implemented / Bounded]`**: Non-blocking `spawn_blocking` Argon2id hashing, versioned expiring AES-256-GCM sessions, fail-closed `RequireRoleLayer`, compile-validated `#[rullst::require_role]`, named `Policy<User, Resource>` decisions, and a feature-gated application JWT policy with required versioned claims, bounded TTL/scopes, strong HS256 keys, `kid` rotation and a revocation-store contract that rejects process-local state in production mode. Authentication, role persistence and resource/tenant lookup remain application boundaries.<br/>🟠 **`[Partial]`**: No built-in shared durable JWT revocation/device adapter exists. Passkey registration/assertion validates the documented ES256/`none`-attestation scope, but normative WebAuthn conformance or adoption of an audited full server library remains required before a general stable claim. |
| **`rullst-security`** | Explicit extended defense-in-depth layers: bounded RASP, authenticated Vault, Login Jail, Secure Headers, rate limiting, DLP and security telemetry. | 🟢 **`[Implemented / Bounded]`**: AES-256-GCM envelopes with rotation/AAD, bounded URI/header/body RASP heuristics, local abuse controls, CSWSH origin guard, OS-random TOTP with SVG enrollment QR, strict JSON syntax/duplicate-key checks, explicit log redaction, file-backed SRI hashes, and a versioned/bounded `LiveSecurityEvent` v1 dashboard envelope. The CLI emits bounded fail-closed evidence and a CycloneDX 1.5 Cargo SBOM; it does not certify the application.<br/>🟢 **`[Implemented / Feature-gated]`**: `redis-rate-limit` provides namespaced atomic Redis fixed-window counters, hashes client keys and exposes an explicit process-local offline mode that production can reject with `require_distributed()`.<br/>🟠 **`[Partial]`**: Recovery-code consumption must be persisted transactionally by the application. Real Redis cross-instance/eviction/failover evidence is still required. CSP nonce composition is shared, but Core and Security are not yet one canonical Server stack; WebSocket CSRF tickets/frame crypto and durable SIEM delivery are not implemented. |
| **`rullst-ai`** | Multi-provider LLM client (Gemini, OpenAI, Claude, DeepSeek, Ollama), prompt injection defenses, PII masking, and guarded local tools. | 🟢 **`[Implemented / Bounded]`**: Guarded `AiClient`, heuristic prompt filter, PII masking, machine-readable provider capabilities, configurable bounded live-request deadlines, a versioned deterministic injection/jailbreak/PII regression corpus, strict URL/resolved-IP/redirect/resource policy plus an opt-in deny-by-default HTTPS fetcher with exact-host allowlist, DNS pinning, proxy bypass, peer verification and streaming limits, and local tool dispatch requiring allowlist, principal authorization, closed bounded JSON, call budget, audit sink, and payload-bound approval for destructive/financial calls.<br/>🟠 **`[Partial]`**: The egress fetcher is not automatically mounted around provider transports or arbitrary application clients; live-origin redirect/stream contracts, live-model/adaptive evals, provider-native tool calling, explicit provider-neutral cancellation, automatic retries, durable production tool auditing, approver authentication, tenant-aware retrieval and domain-specific authorization remain outside the built-in transport.<br/>🟡 **`[Offline Mock]`**: Deterministic offline chat/vision/embedding fallbacks. |
| **`rullst-capital`** | Multi-gateway billing, SaaS MRR/ARR metrics, constant-time webhook signatures, contractor payouts, and an offline NFS-e DPS preview. | 🟢 **`[Implemented / Bounded]`**: Provider-specific payment/payout adapters, pooled HTTP clients, explicit mock credentials, and signature/freshness/replay foundations for the methods documented by each adapter.<br/>🟠 **`[Partial]`**: Uniform live method coverage and durable cross-instance idempotency are incomplete; Alipay RSA2 fails closed.<br/>🟡 **`[Offline Mock]`**: DPS XML generator and deterministic `NfseEnvironment::Mock` fixture.<br/>🔵 **`[Roadmap]`**: Validated XMLDSig/C14N, mTLS transmission and official SEFIN homologation. |
| **`rullst-connect`** | Social login / OAuth2 / OIDC providers (Google, Apple, GitHub, Discord, Auth0, Cognito) with PKCE and rotating JWKS. | 🟢 **`[Implemented / Bounded]`**: OAuth2/OIDC clients with constant-time PKCE comparison, validated discovery, bounded JWKS refresh/cache policy, deterministic mock credentials and a credential-free `UniversalProfile` projection. `ConnectUser` serialization omits access/refresh tokens.<br/>🔵 **`[Roadmap]`**: Message brokers belong in a future messaging boundary rather than being implied by this OAuth-focused crate. |
| **`rullst-iot`** | `no_std` sensor telemetry models and an Ed25519-signed firmware-manifest verification gate. | 🟢 **`[Implemented / Bounded]`**: Ed25519 manifest verification with in-process anti-rollback state, target/hash/length checks, `no_std` telemetry/frame models, a credential-free local HTML snapshot renderer, and a safe telemetry-module CLI scaffold.<br/>🟠 **`[Partial]`**: GPIO state, I2C/Modbus frames, BLE GATT records, RSSI topology, power recommendation and Digital Twin JSON are data/helpers only, not hardware, network or realtime drivers.<br/>🟡 **`[Simulador Dev]`**: Deterministic MQTT/HSM/PQC fixtures require `feature = "experimental-simulators"` and never represent those production capabilities.<br/>🔵 **`[Roadmap]`**: Persistent counter/boot integration, firmware download/flashing, MQTT/CoAP transports, hardware drivers/HSM and audited ML-KEM. |
| **`rullst-mail`** | Transactional email engine with Resend, SendGrid, Postmark, optional SMTP, offline fixtures and an explicit SES-proxy boundary. | 🟢 **`[Implemented / Bounded]`**: Mandatory pre-flight pipeline, anti-CRLF validation, bounded disposable-domain/security/DLP heuristics, provider-specific transports, escaped `make:mail` scaffolds and expiring purpose-bound HMAC tracking tokens.<br/>🟠 **`[Partial]`**: Scheduling and attachment parity vary by transport; tenant selection is explicit and in-process; tracking payloads are authenticated but not confidential. Direct AWS SES v2 fails closed because SigV4 is not implemented; `AwsSesDriver` supports only offline fixtures or an explicit trusted bearer proxy.<br/>🟡 **`[Offline Mock]`**: Memory/Log plus empty or `mock_*` provider credentials. |
| **`rullst-studio`** | Local Developer Control Room (`http://127.0.0.1:5555`), clean route navigation, live system telemetry visualizers. | 🟢 **`[Implemented / Bounded]`**: Local control center, `RadarSnapshot` telemetry, database/migration surfaces when configured, and explicit `Unavailable` states for unconnected probes. |
| **`rullst-nexus`** | Auto-generated Admin CMS (`/nexus`), dynamic model CRUD, AI Admin Assistant (`/nexus/chat`), SOC Threat Radar. | 🟢 **`[Implemented / Bounded]`**: `#[derive(Nexus)]` emits registered named-field metadata with inferred primitive or explicit semantic widgets; the panel provides parameterized CRUD/search/sort/pagination plus bounded selected-record delete/deactivate. Construction is fail-closed, requires an authentication policy and admin role layer, enforces server-side field policy, and escapes record/model metadata on audited paths. Deactivation requires a writable Boolean `is_active`/`active`; host identity, tenant ownership and database privileges remain application contracts. |
| **`rullst-macros`** | Procedural macros (`html!`, `rullst::model`, `rullst::runtime::main`) and compatibility helpers. | 🟢 **`[Implemented / Bounded]`**: Compile-time `html!` escaping with explicit `RawHtml`, model/runtime macros, and `trybuild` diagnostics.<br/>🟠 **`[Partial]`**: `server_function` preserves typed signatures, but browser argument transport and matching server-side RPC registration are not end-to-end. |
| **`cargo-rullst`** | Developer CLI toolkit, scaffolding generators (`make:*`), project blueprints, AST IDOR static route scanner. | 🟢 **`[Implemented / Bounded]`**: Interactive wizard, generators, heuristic IDOR scanner, CycloneDX exporter, toolchain doctor and a fail-closed Academy evidence diagnostic that explicitly does not certify a deployment. `make:chat-session` emits registered SQLx or Turso-primary models, reversible migrations and application-owned bounded chat memory; materialized contracts run persistent mock conversations on both backends and prove collision refusal.<br/>🟢 **`[Implemented / Bounded]`**: The LMS starter supplies bounded curriculum, school-scoped learning/assessment/publication/progress/completion, roles, leaderboard, automation/outbox/workers, localized in-app notifications and a minimized privacy-request foundation. Its SSR catalog performs limited, ORM-parameterized title/category filtering; generated auth/catalog/course/player shells consume the Core CSP nonce without remote page dependencies or inline style attributes and include keyboard landmarks, visible focus and reduced-motion handling. Privacy claims use exact leases, retry/dead-letter with a hard ten-attempt ceiling, actor/digest-bound completion and a supervised static-dispatch executor with an explicit protocol-only mock; the product must still supply the adapter that performs application-specific export/deletion/anonymization. Materialized SQLite exercises catalog escaping/nonce, privacy hard limits and the documented vertical/cross-school boundaries. Detached `--lms-modules auth`, `auth,learning` and `auth,learning,assessment` profiles remain small compiling foundations; the assessment profile grades versioned quizzes authoritatively without pulling score/leaderboard/outbox verticals. The complete starter is the default.<br/>🟠 **`[Partial]`**: Other detached combinations, profile hot reload, attachments/media, advanced/localized search, captions/transcripts, WCAG/browser evidence, distributed failover, PostgreSQL/MySQL isolation, visual authoring, exported telemetry and the separately operated Academy remain roadmap or release-engineering work. |

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
  identifiers use the bounded ASCII grammar and pgvector helpers accept only
  finite canonical numeric vectors. Methods explicitly suffixed/named `raw`
  remain caller-owned escape hatches rather than an injection-safety claim.
* `String` and `Option<String>` fields annotated with `#[orm(encrypted)]` are encrypted before generated ORM writes and decrypted after generated model reads using AES-256-GCM. Randomized ciphertext cannot be filtered, ordered, grouped, or explicitly selected by generated query-builder methods; use a separately reviewed blind index when equality lookup is required. Raw SQL remains an explicit, non-transparent escape hatch.

### 5.3. Tenant Scope Contract

* A SQLx model declaring `#[orm(tenant_column = "tenant_id")]` must have a
  persisted `String`, `i32`, `f64`, or `bool` tenant field. The derive rejects a
  missing or unsupported field type.
* Generated queries fail closed when called outside `with_tenant(...)` and bind
  the active tenant inside the scope. Generated full/partial updates and
  instance delete/restore paths reject a model from another tenant.
* `Model::unscoped()` is the explicit global escape hatch. Deciding who may use
  it, deriving tenant identity from authenticated state, and database-level RLS
  remain host responsibilities.

### 5.4. Polyglot Persistence Boundary

* Optional persistence adapters are disabled by default and selected with
  `mongodb`, `duckdb`, `turso`, `surrealdb`, or the `polyglot` convenience
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
* `TursoStore` uses the official remote-only `libsql` driver for edge SQL,
  prepared parameters, bounded materialization, transactions and ordered
  checksummed migrations. Empty or `mock_*` endpoints use a one-connection
  SQLite in-memory fallback that exercises real SQL without pretending to be a
  remote replica. HTTPS/`libsql://` is required outside explicitly enabled
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

---

## 💳 6. Billing, Payments & Fiscal Engine (`rullst-capital`)

`rullst-capital` exposes bounded billing-provider and payout-provider adapters,
plus an offline-only Brazilian digital-invoicing preview (NFS-e Nacional).

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
corresponding helpers. It does not infer ownership, team membership, currency,
payment methods, usage or database-backed quotas.

### 6.2. Invoice Rendering

`Invoice::generate_html` escapes every application-supplied textual field. It
renders HTML only: native PDF generation and automatic delivery after a payment
event are not part of the current contract.

### 6.3. Webhook Signature Verification
* Built-in webhook adapters use provider-appropriate cryptographic verification;
  equality checks for derived signatures are constant-time where applicable.
* Timestamped protocols enforce a bounded freshness window. Applications still
  need durable event-id idempotency across processes.

### 6.4. NFS-e Nacional Specification (`FiscalEngine`)
* 🟢 **`[Implementado]` Offline DPS Generator:** Serializes standardized XML DPS documents with proper XML character escaping and entity validation.
* 🟡 **`[Simulado]` Offline Mock Environment:** `NfseEnvironment::Mock` produces deterministic test fixtures for local sandboxing.
* 🔵 **`[Roadmap]` Official SEFIN Homologation & Production:**
  * Canonicalization Method: W3C XML C14N (`http://www.w3.org/TR/2001/REC-xml-c14n-20010315`).
  * Signature Method: RSA-SHA256 enveloped XMLDSig using ICP-Brasil A1 certificates (PKCS#12).
  * Transmission Transport: Mutual TLS (mTLS) against the National SEFIN Gateway endpoints.

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
* The database browser is read/filter only and accepts a deliberately narrow
  ASCII SQL-identifier boundary. The ER diagram inspects SQLite, PostgreSQL,
  MySQL, or MariaDB metadata with bound lookup values and strict normalized
  Mermaid identifiers. Swagger requires an application-supplied `OpenApi`.
* Request SSE records method, URI, status, and latency without bodies or headers.
  Environment values are redacted by default and the typed config projection
  never renders connection URLs, filesystem paths, cookies, tokens, or
  credentials. Database flag changes remain subject to `DbFeatureDriver` cache
  TTL, and SQLite queues do not retain completed-job history.
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
