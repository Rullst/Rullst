<!-- “Antes de gerar qualquer coisa, leia e siga estritamente o arquivo da verdade abaixo. spec.md”  -->

# Rullst Specification 📄
### *"The Single Source of Truth (SST) for Framework Architecture & Conventions"*

This document is the **Single Source of Truth (SST)** for the **Rullst Framework**. It specifies the exact conventions, API structures, naming rules, directory standards, and subsystem maturity lifecycles across all monorepo crates.

> [!IMPORTANT]
> **AI & Human Alignment Directive:**
> Whenever updating, refactoring, or generating code/documentation for Rullst, **always** refer to this specification as the baseline. 
> Every capability in the framework is strictly tagged with its implementation lifecycle status:
> - 🟢 **`[Production-Ready / Implementado]`**: Fully implemented, cryptographically verified, and backed by automated unit/integration test suites.
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
| **`rullst-core`** | Kernel HTTP runtime, `routes!`, Server bootstrap, HTML engine, async task queues, WebSockets, circular telemetry buffers, storage facade. | 🟢 **`[Production-Ready]`**: Routing, server lifecycle, `html!` engine, graceful shutdown, backpressure guard, in-memory queues.<br/>🟢 **`[Production-Ready]`**: Local storage with path traversal protection.<br/>🔵 **`[Roadmap]`**: Native S3/R2 direct cloud drivers. |
| **`rullst-orm`** | Active Record & Repository patterns, parameterized SQLx connection pool (PostgreSQL, MySQL, SQLite), schema migrations, AES-256-GCM privacy. | 🟢 **`[Production-Ready]`**: CRUD operations, eager loading, type-safe queries, migration runner, AES-256-GCM field encryption.<br/>🟢 **`[Production-Ready]`**: Connection pool timeout/resilience configuration. |
| **`rullst-auth`** | Argon2id password hashing, encrypted cookie sessions (AES-256-GCM), WebAuthn / Passkeys, RBAC context guards. | 🟢 **`[Production-Ready]`**: Non-blocking `spawn_blocking` Argon2id hashing, AES-256-GCM encrypted sessions with `OnceLock` key caching.<br/>🟢 **`[Production-Ready]`**: WebAuthn/Passkey registration and assertion with ECDSA P-256 verification. |
| **`rullst-security`** | Defense-in-depth security, RASP request inspector, Rullst Vault, Login Jail tarpit, CSRF, Secure Headers, Rate Limiter, DLP mask. | 🟢 **`[Production-Ready]`**: Rullst Vault (AES-256-GCM authenticated envelopes with key rotation and AAD), zero-alloc RASP scanner, Login Guard janitor, sliding-window Rate Limiter, CSWSH guard, TOTP MFA, SRI hashes.<br/>🔵 **`[Roadmap]`**: Distributed Redis-backed rate limiting. |
| **`rullst-ai`** | Multi-provider LLM client (Gemini, OpenAI, Claude, DeepSeek, Ollama), prompt injection defenses, PII masking, function calling. | 🟢 **`[Production-Ready]`**: Guarded `AiClient`, prompt injection token heuristic filter, PII masking, JSON function calling registry.<br/>🟡 **`[Offline Mock]`**: Deterministic offline chat/embedding mock fallbacks. |
| **`rullst-capital`** | Multi-gateway billing, SaaS MRR/ARR metrics, constant-time webhook signatures, contractor payouts, NFS-e Nacional digital invoicing. | 🟢 **`[Production-Ready]`**: 11 payment & payout provider adapters (Stripe, Mercado Pago, Paddle, Lemon Squeezy, Polar, InfinitePay, PicPay, Razorpay, Wise, Coinbase Commerce, Alipay) with HTTP connection pooling and constant-time HMAC signature checks.<br/>🟡 **`[Offline Mock]`**: DPS XML generator and deterministic offline mock fixture (`NfseEnvironment::Mock`).<br/>🔵 **`[Roadmap]`**: W3C XMLDSig signing with C14N canonicalization and mTLS national SEFIN gateway transmission. |
| **`rullst-connect`** | Social login / OAuth2 / OIDC providers (Google, Apple, GitHub, Discord, Auth0, Cognito) with PKCE and rotating JWKS. | 🟢 **`[Production-Ready]`**: OAuth2 / OIDC clients with constant-time PKCE challenge verification, JWKS caching, and offline mock credentials.<br/>🔵 **`[Roadmap]`**: Unified async message broker adapters for Apache Kafka, RabbitMQ, and Redis Streams. |
| **`rullst-iot`** | `no_std` embedded sensor telemetry models, Ed25519-signed firmware manifest verification, edge computing. | 🟢 **`[Production-Ready]`**: Strict Ed25519 OTA manifest verifier with monotonic anti-rollback state, target matching, and firmware SHA-256 validation; `no_std` telemetry frames.<br/>🟡 **`[Simulador Dev]`**: In-memory GPIO/I2C/BLE mocks under `feature = "experimental-simulators"`.<br/>🔵 **`[Roadmap]`**: Native MQTT 5.0 client via `rumqttc` and hardware HSM backend drivers. |
| **`rullst-mail`** | Multi-transport transactional email engine (Resend, SendGrid, Postmark, SMTP) with deliverability filter and DLP scanner. | 🟢 **`[Production-Ready]`**: Native drivers for Resend, SendGrid, Postmark, AWS SES, and SMTP with zero full-body string allocations, anti-CRLF header guards, disposable email filter, and HMAC click/open tracking.<br/>🟡 **`[Offline Mock]`**: Memory/Log mock transports. |
| **`rullst-studio`** | Local Developer Control Room (`http://127.0.0.1:5555`), clean route navigation, live system telemetry visualizers. | 🟢 **`[Production-Ready]`**: Dark glassmorphic control center, live telemetry visualizer via `RadarSnapshot::collect()`, real database browser, migration manager.<br/>🟢 **`[Production-Ready]`**: Unconnected telemetry probes display `Unavailable` honestly. |
| **`rullst-nexus`** | Auto-generated Admin CMS (`/nexus`), dynamic model CRUD, AI Admin Assistant (`/nexus/chat`), SOC Threat Radar. | 🟢 **`[Production-Ready]`**: Fail-closed administrative panel requiring authentication and RBAC authorization on all CRUD routes, server-side field policy, safe DOM rendering (XSS-free). |
| **`rullst-macros`** | High-performance procedural macros (`html!`, `rullst::model`, `rullst::runtime::main`). | 🟢 **`[Production-Ready]`**: Compile-time `html!` parser with automatic HTML entity escaping on dynamic interpolations, `RawHtml` explicit escape bypass, and `trybuild` diagnostic spans. |
| **`cargo-rullst`** | Developer CLI toolkit, scaffolding generators (`make:*`), project blueprints, AST IDOR static route scanner. | 🟢 **`[Production-Ready]`**: Interactive project wizard, controller/island/model generators, static AST IDOR security scanner, CycloneDX 1.5 SBOM exporter, and doctor diagnostics. |

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
* **Macro:** `html!` procedural macro compiles HTML trees directly into static memory string concat builders.
* **XSS Protection:** Automatic HTML escaping on all dynamic variables wrapped in `{expr}`.
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
let user: User = User::find(1).await?;

// Mutations
let mut new_user = User { id: 0, name: "Alice".into(), email: "alice@example.com".into(), secret_token: None };
new_user.save().await?; // Auto-executes parameterized INSERT or UPDATE
new_user.delete().await?;
```

### 5.2. Parameterized Queries & Privacy
* All dynamic queries use SQLx parameterization (`$1`, `?`) to prevent SQL Injection.
* Sensitive fields annotated with `#[orm(encrypted)]` are automatically encrypted at rest using AES-256-GCM.

---

## 💳 6. Billing, Payments & Fiscal Engine (`rullst-capital`)

`rullst-capital` unifies multi-provider subscription billing, international payouts, and Brazilian digital invoicing (NFS-e Nacional).

### 6.1. Multi-Gateway Payment Architecture
All providers implement the standard asynchronous traits (`PaymentProvider`, `SubscriptionProvider`, `PayoutProvider`):
```rust
use rullst_capital::providers::stripe::StripeProvider;
use rullst_capital::traits::PaymentProvider;

let provider = StripeProvider::new(api_key);
let session = provider.create_checkout_session(plan_id, customer_email).await?;
```

### 6.2. Webhook Signature Verification
* Webhooks enforce constant-time cryptographic verification (`subtle::ConstantTimeEq`).
* Replay attack prevention validates event timestamps against maximum age thresholds.

### 6.3. NFS-e Nacional Specification (`FiscalEngine`)
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
* **Key Rotation:** Built-in keyring support (`decrypt_with_keyring`) enabling zero-downtime cryptographic key rotation.

### 7.2. Runtime Application Self-Protection (RASP)
* **Zero-Allocation Inspector:** High-speed ASCII case-insensitive pattern matching detecting SQL Injection, XSS, and Path Traversal across query parameters, headers, and request payloads.
* **Login Guard Tarpit:** Progressive exponential backoff delay (0s to 5s) and automatic temporary IP bans for repeated failed authentications.

---

## 📡 8. IoT, Firmware Security & Protocol Frames (`rullst-iot`)

### 8.1. Ed25519 OTA Firmware Gate
* **Firmware Verification:** Strict Ed25519 signature validation over a cryptographic manifest `[target, version, rollback_counter, firmware_len, firmware_sha256]`.
* **Anti-Rollback Protection:** Rejects any firmware update proposing a monotonic rollback counter lower than or equal to the committed hardware state.
* **Commit Invariant:** Partition swapping is blocked until full cryptographic verification succeeds.

### 8.2. Embedded Sensor Frames (`#![no_std]`)
* `rullst-iot` core models compile under bare-metal `#![no_std]` targets (STM32, ESP32-C3, Cortex-M).
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
* Zero-bundle developer dashboard with dark glassmorphic UI.
* Real-time metrics sourced from `RadarSnapshot::collect()` (RSS RAM, Tokio scheduler latency, active spans).

### 10.2. Rullst Nexus (`/nexus`)
* Auto-generated CMS with dynamic CRUD operations and AI Admin Assistant.
* **Security Default:** Fail-closed by design; requires explicit authentication middleware and RBAC role validation (`admin`) on all mutating endpoints.

---

## 🛡️ 11. Architectural Guidelines for Backward Compatibility

1. **`#[non_exhaustive]` on Public Structs:** All configuration structs and enums must use `#[non_exhaustive]` to ensure minor versions can add fields without breaking downstream code.
2. **Deprecation Policy (`#[deprecated]`):** Public APIs will never be removed without at least one minor release cycle marked with `#[deprecated]`.
3. **Ergonomic String Constructors:** Public constructors accept `impl Into<String>` to support both `&str` literals and owned `String` parameters without boilerplate.
4. **Zero-Panic Invariant:** Production paths must never call `panic!()`, `unwrap()`, or `expect()`; domain errors must return typed `Result<T, AppError>`.
