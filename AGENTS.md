# AI Agents & Assistants Guidelines for Rullst Framework

Welcome! If you are an autonomous AI agent or coding assistant contributing to **Rullst**, this document is your primary directive manual. 

Rullst is designed from the ground up to be **AI-Native**, meaning its architecture explicitly avoids runtime "magic" reflection in favor of strong compile-time typing, macro-driven code generation, and explicit APIs. This makes the codebase easy to reason about, optimize, and safely refactor.

---

## 📜 1. Single Source of Truth (SST)
The document [`docs/src/spec.md`](docs/src/spec.md) is our absolute law. Always reference `docs/src/spec.md` before proposing or executing architectural changes, file scaffoldings, or API modifications.

---

## 🏛️ 2. Crate Architecture Overview

The Rullst framework is organized into decoupled, high-performance crates:

| Crate | Responsibilities |
| :--- | :--- |
| **`rullst-core`** | Runtime-only-by-default server, Tokio integration, `routes!`, queues/realtime, `RadarSnapshot`, and `SpanCollector`; ORM/SQLite queue support is feature-gated. |
| **`rullst-orm`** | Parameterized SQLx database pool, Active Record & Repository patterns, dynamic schema inspector, and a v13 transactional partial-update candidate under validation; see the SST for full-row persistence and rollback semantics. |
| **`rullst-auth`** | Argon2 password hashing, encrypted session management, passkeys and an optional v13 shared PostgreSQL ceremony candidate, RBAC helpers, and OAuth2/OIDC re-exports. |
| **`rullst-security`** | RASP/WAF defense-in-depth, strict secure headers, Login Jail, DLP, honeypots, RBAC, and security telemetry. |
| **`rullst-ai`** | Provider-agnostic LLM client (Gemini, OpenAI, Claude, DeepSeek, Ollama), prompt injection filter, PII masking. |
| **`rullst-capital`** | Multi-provider payment and payout adapters, webhook verification, SaaS analytics, an offline NFS-e preview and bounded local fiscal preparation; live transmission and fiscal authorization remain disabled pending external validation. |
| **`rullst-connect`** | OAuth2/OIDC and social-login providers; brokered messaging is deliberately outside this identity-focused crate. |
| **`rullst-messaging`** | Bounded broker-neutral envelopes, idempotent publication, consumer groups, leases, retry/DLQ, canonical wire/trace contracts, a deterministic process-local broker, opt-in durable local SQLite state with explicit encrypted content, and an opt-in ORM outbox relay, and a standalone Redis Streams candidate with Rullst-owned fenced delivery indexes under hosted validation; other remote adapters remain roadmap work. |
| **`rullst-iot`** | `no_std` telemetry/frame helpers, bounded MQTT 5 PUBLISH and CoAP request encoders, Ed25519-signed OTA manifest verification, and a caller-provided durable rollback-counter CAS contract. Concrete counter storage, network transports/session state, HSM, PQC, flashing, and bootloader integration are roadmap work. |
| **`rullst-privacy`** *(unpublished v13 package candidate)* | Optional proportional age policies, native declarations, authenticated challenge transport, signed threshold attestations and trusted-clock decisions. Replay storage supports shared-local SQLite or an authoritative PostgreSQL database; independent versioned optional consent/withdrawal has a shared-local SQLite adapter. Opt-in CLI consumers compose SaaS/LMS authentication, optional processing and a bounded own-account profile export. Live age providers, facial models, verified guardianship, deployment-specific failover and broader privacy workflows remain roadmap work. |
| **`rullst-mail`** | Templated transactional email engine (Resend, SendGrid, Postmark, SMTP) with background delivery, opt-in attachment inspection, recipient suppression, and minimized delivery observations. |
| **`rullst-studio`** | Developer Control Room (`http://127.0.0.1:5555`), clean routes (`/studio/*`), dark glassmorphic UI, non-mocked telemetry. |
| **`rullst-nexus`** | Auto-generated Admin CMS (`/nexus`), model CRUD interfaces, AI Admin Assistant (`/nexus/chat`), SOC Threat Radar. |
| **`rullst-macros`** | Procedural macros (`html!`, `rullst::model`, `rullst::runtime::main`). |
| **`cargo-rullst`** | Developer CLI scaffold generator (`make:*` commands), AST IDOR scanner, and 1-Click cloud deployer. |
| **`rullst-labs`** *(unpublished v13 candidate)* | Trusted bounded exercise/authorization/grading contracts, optional encrypted shared-local SQLite jobs, fenced leases/cancellation/retention and pinned controller receipts. It never executes learner code. PR #228 passed hosted/platform and installed-archive source admission. Final release admission remains outstanding. |
| **`rullst-media`** *(unpublished v13 candidate)* | Optional Bunny Stream lifecycle, scoped upload/playback grants, exact-body notifications, shared-local SQLite reconciliation and browser upload module. PR #227 passed hosted/platform and installed-archive source admission. Actual provider/CDN interoperability remains unvalidated; no release admission or blueprint/facade default is implied. |
| **`rullst-supervision`** *(unpublished v13 implementation candidate)* | Optional exam/parental contracts, exact collection acknowledgement, typed browser/capture observations, bounded camera-presence/audio-activity adapter orchestration and shared-local SQLite authority/session/retention state. The host owns authenticated membership, capture permission, models, reviewer workflow and verified relationships. No media model, automatic grading or device-wide control is included. PR #226 is merged; hosted checks and retrospective installed-archive validation passed. Final release admission remains separate. |
| **`rullst-labs-runner`** *(unpublished experimental v13 candidate)* | Separately deployed Linux Rust-to-Wasm compiler/Wasmi controller with mandatory observed cgroups, namespaces, seccomp and Landlock restrictions. It must not run in the web process or give workers keys, graders or control sockets. PR #228 passed source admission with real isolated acceptance. Final release admission and independent isolation review remain outstanding; the non-required patch-coverage gap is recorded in the profile evidence. |

---

## 🛡️ 3. Core Coding Directives & Invariants

### 3.1. Zero-Panic Policy in Production Code
- Never use `panic!()`, `unwrap()`, or `expect()` in non-test production paths.
- Always use typed error enums (`AppError`, `CapitalError`, `OrmError`, `FiscalError`, etc.) for graceful degradation and structured error responses.
- In `#[test]` modules, `unwrap()` and `expect()` are fully allowed and encouraged for concise assertions.

### 3.2. Static Dispatch & Constructor Ergonomics
- Prefer static dispatch (`impl Trait` or generic parameters) over `dyn Trait`.
- Public constructors (`new`) and helper methods taking string parameters should accept `impl Into<String>` (e.g. `pub fn new(api_key: impl Into<String>)`) to permit ergonomic passing of both string literals (`&str`) and owned `String` instances without manual conversion boilerplate.
- Explicit concrete types improve compile-time optimization, inline expansion, and AI context tracking.

### 3.3. HTML Macro Rules (`html!`)
- Boolean attributes inside the `html!` macro must be explicitly quoted (e.g. `required="true"`, `disabled="true"`).
- Prefer zero-bundle HTMX + Tailwind CSS server-side rendering over heavy client-side JS bundles.

### 3.4. Security & Cryptographic Invariants
- All dynamic SQL inputs must use SQLx parameterization (`sqlx::query(...)`) or strict alphanumeric sanitization (`sanitize_identifier`).
- CSRF middleware (`Double-Submit Cookie`), OWASP Secure Headers (`SecureHeadersLayer`), and WAF middleware are mandatory for production endpoints.
- Webhook signature verifications must enforce constant-time comparisons (`subtle::ConstantTimeEq` or cryptographic HMAC verification) to eliminate timing attacks.
- Parameterized data routes (`/:id`, `/{id}`) must enforce ownership validation via `RbacGuard::authorize_owner_or_role` or `UserContext`.
- Avoid introducing new `unsafe` blocks unless strictly necessary for OS FFI, and document safety invariants explicitly.

### 3.5. External Providers & Mock Fallbacks
- All external service integrations (Payment Gateways, OAuth2 providers, Email transports, LLM APIs) must implement a deterministic offline mock fallback when initialized with empty or `mock_*` credentials.
- This ensures test suites, local sandboxes, and developer tools run reliably offline without requiring live third-party API credentials.

### 3.6. Studio & Observability Directives
- Studio sub-routes must follow the clean URL standard without `/tools/` (e.g. `/studio/radar`, `/studio/capital`, `/studio/security`, `/studio/traces`).
- Studio visualizers must use real runtime telemetry (`RadarSnapshot::collect()`, `SpanCollector`, real DB queries). Never hardcode mock data in Studio or Nexus.
- All Studio pages must render using the unified dark glassmorphic `studio_layout` design system (`slate-950` palette, live status pulse badges).

### 3.7. Modular Code & File Size Standard (< 500 Lines Target)
- Keep source files focused, decoupled, and concise (target max 500 lines per file).
- Avoid monolithic single-file bloat by decomposing large modules into dedicated sub-module directories using Rust's `mod` system (e.g. `data_browser/` containing `db.rs`, `layout.rs`, `handlers.rs`, `mod.rs`).
- Smaller files improve compile times, IDE performance, and AI context tracking precision.

### 3.8. Cross-Platform Build Hygiene & Windows File Locks
- On Windows systems, procedural macro dynamic libraries (`.dll` files in `target/debug/deps/`) remain locked by running processes.
- Always ensure active background compiler or test processes are terminated before executing destructive workspace operations like `cargo clean`.

#### Local disk-space safety

- On a developer workstation, measure space **available to the current user**
  (for example, `df -h`), not total free blocks that include the filesystem's
  administrator reserve. Check the filesystems holding the workspace, Cargo
  target directory and temporary files before builds, tests or large downloads.
- Keep at least **12 GiB available** as an operational reserve. At **15 GiB**,
  intervene: do not start another disk-intensive job; stop/pause agent-owned
  builds if space is declining, review disposable artifacts and notify the user.
  Stop agent-owned disk-consuming jobs at 12 GiB; do not exhaust the reserve.
- A build may start only with at least 15 GiB available **and** enough additional
  headroom for its estimated peak while retaining the 12 GiB reserve. An unknown
  cold all-feature/native build is not a small job; prefer hosted CI when its
  peak cannot be bounded. Recheck during long-running jobs and after each batch.
- Prefer targeted, reusable builds over simultaneous local matrices. Clean only
  verified generated artifacts, after checking for active users of that target.
  Never delete source, user files, credentials or database state, and never
  reduce the OS filesystem reserve to make a build fit.
- These thresholds are an agent operating policy, not an OS disk quota or a
  guarantee against writes by other processes. Report that limitation honestly.

### 3.9. Official Inquiries & Vulnerability Disclosure
- Official Framework Email: `officialrullst@gmail.com`.
- Security vulnerabilities must be handled via coordinated private disclosure directly to the core team.

### 3.10. Git Governance & Anti-Fluff Directives
- **Conventional Commits Only**: All commit messages created by AI agents or human contributors must strictly follow the format: `<type>(<scope>): <summary>` (e.g. `feat(orm): add json filter support`, `fix(auth): offload password hashing to spawn_blocking`, `test(radar): add telemetry span assertions`).
- **No Marketing Fluff or Hallucinated Summaries**: Prohibit AI-generated multi-paragraph essays in commit bodies. Commit messages must be concise, purely technical, and match the exact diff.

---

## 🧪 4. Testing, Verification & Release Workflow

### 4.1. Pre-Flight Verification Standard
Before declaring any task or feature complete, agents must execute and verify the following trifecta:
1. **Full Test Suite**: `cargo test --workspace --all-features` (ensure 100% pass rate across unit, integration, and doc tests).
2. **Strict Linter**: `cargo clippy --workspace --all-features -- -D warnings` (zero warnings allowed).
3. **Format Integrity**: `cargo fmt --all` (enforce standard Rust formatting).

### 4.2. Crates.io Topological Release Order
When publishing or preparing multi-crate releases to crates.io, publish in strict topological dependency order (allowing brief indexing intervals between packages):
```bash
# Step 1: Base Procedural Macros
cargo publish -p rullst-macros
cargo publish -p rullst-orm-macros

# Step 2: ORM then Core (Core has an optional ORM dependency)
cargo publish -p rullst-orm
cargo publish -p rullst-core
cargo publish -p rullst-messaging

# Step 3: Domain & Service Crates
cargo publish -p rullst-connect
cargo publish -p rullst-iot
cargo publish -p rullst-security
cargo publish -p rullst-ai
cargo publish -p rullst-capital
cargo publish -p rullst-mail
cargo publish -p rullst-auth
cargo publish -p rullst-privacy

# Step 4: Visual Dashboards & Admin Interfaces
cargo publish -p rullst-nexus
cargo publish -p rullst-studio

# Step 5: Umbrella Metapackage & Developer CLI
cargo publish -p rullst
cargo publish -p cargo-rullst
```

The machine-readable source of truth is `.github/release-order.json`; the
release workflow validates it against `cargo metadata` before packaging.

### 4.3. CLI Scaffolding First
- Prefer CLI scaffolding generators (`cargo rullst make:*`) when adding new entities, controllers, or migrations to ensure architectural consistency.
