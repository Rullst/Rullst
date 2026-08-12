# AI Agents & Assistants Guidelines for Rullst Framework

Welcome! If you are an autonomous AI agent or coding assistant contributing to **Rullst**, this document is your primary directive manual. 

Rullst is designed from the ground up to be **AI-Native**, meaning its architecture explicitly avoids runtime "magic" reflection in favor of strong compile-time typing, macro-driven code generation, and explicit APIs. This makes the codebase easy to reason about, optimize, and safely refactor.

---

## 📜 1. Single Source of Truth (SST)
The document [`docs/spec.md`](docs/spec.md) is our absolute law. Always reference `docs/spec.md` before proposing or executing architectural changes, file scaffoldings, or API modifications.

---

## 🏛️ 2. Crate Architecture Overview

The Rullst framework is organized into decoupled, high-performance crates:

| Crate | Responsibilities |
| :--- | :--- |
| **`rullst-core`** | Server runtime, Tokio executor integration, `routes!` macro, `RadarSnapshot` telemetry & `SpanCollector`. |
| **`rullst-orm`** | Parameterized SQLx database pool, Active Record & Repository patterns, dynamic schema inspector. |
| **`rullst-auth`** | JWT authentication, Argon2 password hashing, session management, OAuth2 integration. |
| **`rullst-security`** | RASP deep inspection, OWASP Secure Headers A+, Login Jail tarpit, DLP interceptor, honeypot traps, WAF middleware. |
| **`rullst-ai`** | Provider-agnostic LLM client (Gemini, OpenAI, Claude, DeepSeek, Ollama), prompt injection filter, PII masking. |
| **`rullst-capital`** | Real-time SaaS MRR/ARR analytics, Stripe/LemonSqueezy webhook audit log ledger. |
| **`rullst-connect`** | Enterprise message queues (RabbitMQ, Redis Streams, Kafka), WebSockets sync, SSE event streams. |
| **`rullst-iot`** | High-throughput MQTT 5.0 broker client, industrial edge sensor ingestion, zero-copy packet parser. |
| **`rullst-mail`** | Templated transactional email engine (Resend, SendGrid, Postmark, SMTP) with background delivery. |
| **`rullst-studio`** | Developer Control Room (`http://127.0.0.1:5555`), clean routes (`/studio/*`), dark glassmorphic UI, non-mocked telemetry. |
| **`rullst-nexus`** | Auto-generated Admin CMS (`/nexus`), model CRUD interfaces, AI Admin Assistant (`/nexus/chat`), SOC Threat Radar. |
| **`rullst-macros`** | Procedural macros (`html!`, `rullst::model`, `rullst::runtime::main`). |
| **`cargo-rullst`** | Developer CLI scaffold generator (`make:*` commands), AST IDOR scanner, and 1-Click cloud deployer. |

---

## 🛡️ 3. Core Coding Directives & Invariants

### 3.1. Zero-Panic Policy in Production Code
- Never use `panic!()`, `unwrap()`, or `expect()` in non-test production paths.
- Always use the typed `AppError` enum for graceful degradation and structured error responses.
- In `#[test]` modules, `unwrap()` and `expect()` are fully allowed and encouraged for assertions.

### 3.2. Static Dispatch over Dynamic
- Prefer static dispatch (`impl Trait` or generic parameters) over `dyn Trait`.
- Explicit concrete types improve compile-time optimization, inline expansion, and AI context tracking.

### 3.3. HTML Macro Rules (`html!`)
- Boolean attributes inside the `html!` macro must be explicitly quoted (e.g. `required="true"`, `disabled="true"`).
- Prefer zero-bundle HTMX + Tailwind CSS server-side rendering over client-side JS bundles.

### 3.4. Security Invariants
- All dynamic SQL inputs must use SQLx parameterization (`sqlx::query(...)`) or strict alphanumeric sanitization (`sanitize_identifier`).
- CSRF middleware (`Double-Submit Cookie`), OWASP Secure Headers (`SecureHeadersLayer`), and WAF middleware are mandatory for production endpoints.
- Parameterized data routes (`/:id`, `/{id}`) must enforce ownership validation via `RbacGuard::authorize_owner_or_role` or `UserContext`.
- Avoid introducing new `unsafe` blocks unless strictly necessary for OS FFI, and document safety invariants explicitly.

### 3.5. Studio & Observability Directives
- Studio sub-routes must follow the clean URL standard without `/tools/` (e.g. `/studio/radar`, `/studio/capital`, `/studio/security`, `/studio/traces`).
- Studio visualizers must use real runtime data (`RadarSnapshot::collect()`, `SpanCollector`, real DB queries). Never hardcode mock data in Studio or Nexus.
- All Studio pages must render using the unified dark glassmorphic `studio_layout` design system (`slate-950` palette, live status pulse badges).

### 3.6. Modular Code & File Size Standard (< 500 Lines Target)
- Keep source files focused, decoupled, and concise (target max 500 lines per file).
- Avoid monolithic single-file bloat by decomposing large modules into dedicated sub-module directories using Rust's `mod` system (e.g. `data_browser/` containing `db.rs`, `layout.rs`, `handlers.rs`, `mod.rs`).
- Smaller files improve compile times, IDE performance, and AI context tracking precision.

### 3.7. Official Inquiries & Vulnerability Disclosure
- Official Framework Email: `officialrullst@gmail.com`.
- Security vulnerabilities must be handled via coordinated private disclosure directly to the core team.

---

## 🧪 4. Testing & Verification Workflow

1. Always run `cargo test -p <target-crate>` after modifying codebase files.
2. Verify that existing API signatures and public contracts remain unbroken.
3. Prefer CLI scaffolding generators (`cargo rullst make:*`) when adding new entities, controllers, or migrations.
