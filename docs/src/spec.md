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
| **`rullst-core`** | Kernel HTTP runtime, `routes!`, Server bootstrap, HTML engine, async task queues, WebSockets, circular telemetry buffers, storage facade, and the default baseline CSRF/WAF/header/PII stack. | 🟢 **`[Implemented / Bounded]`**: Routing, server lifecycle, `html!` engine, graceful shutdown, backpressure guard, in-memory queues, and local storage with path-traversal protection.<br/>🟢 **`[Implemented / Bounded]`**: Server-mounted baseline security; the extended `rullst-security` stack is composed explicitly.<br/>🔵 **`[Roadmap]`**: Native S3/R2 direct cloud drivers. |
| **`rullst-orm`** | Active Record & Repository patterns, parameterized SQLx connection pool (PostgreSQL, MySQL, SQLite), schema migrations, AES-256-GCM privacy. | 🟢 **`[Implemented / Bounded]`**: CRUD operations, eager loading, type-safe queries, migration runner, versioned field encryption, and connection-pool resilience configuration for supported drivers/features. |
| **`rullst-auth`** | Argon2id password hashing, encrypted cookie sessions (AES-256-GCM), Passkey ceremony foundations, RBAC context guards. | 🟢 **`[Implemented / Bounded]`**: Non-blocking `spawn_blocking` Argon2id hashing and versioned, expiring AES-256-GCM sessions.<br/>🟠 **`[Partial]`**: Passkey registration/assertion validates the documented ES256/`none`-attestation scope, but normative WebAuthn conformance or adoption of an audited full server library remains required before a general stable claim. |
| **`rullst-security`** | Explicit extended defense-in-depth layers: bounded RASP, authenticated Vault, Login Jail, Secure Headers, local Rate Limiter, DLP and security telemetry. | 🟢 **`[Implemented / Bounded]`**: AES-256-GCM envelopes with rotation/AAD, bounded URI/header/body RASP heuristics, local abuse controls, CSWSH guard, TOTP foundation and SRI hashes.<br/>🟠 **`[Partial]`**: CSP nonce composition is shared, but ownership/mounting/telemetry are not yet one canonical Server stack; Core currently owns baseline CSRF/WAF/headers/PII.<br/>🔵 **`[Roadmap]`**: Distributed atomic rate limiting. |
| **`rullst-ai`** | Multi-provider LLM client (Gemini, OpenAI, Claude, DeepSeek, Ollama), prompt injection defenses, PII masking, function calling. | 🟢 **`[Implemented / Bounded]`**: Guarded `AiClient`, heuristic prompt filter, PII masking, JSON tool registry, and provider capability errors.<br/>🟡 **`[Offline Mock]`**: Deterministic offline chat/vision/embedding fallbacks. |
| **`rullst-capital`** | Multi-gateway billing, SaaS MRR/ARR metrics, constant-time webhook signatures, contractor payouts, and an offline NFS-e DPS preview. | 🟢 **`[Implemented / Bounded]`**: Provider-specific payment/payout adapters, pooled HTTP clients, explicit mock credentials, and signature/freshness/replay foundations for the methods documented by each adapter.<br/>🟠 **`[Partial]`**: Uniform live method coverage and durable cross-instance idempotency are incomplete; Alipay RSA2 fails closed.<br/>🟡 **`[Offline Mock]`**: DPS XML generator and deterministic `NfseEnvironment::Mock` fixture.<br/>🔵 **`[Roadmap]`**: Validated XMLDSig/C14N, mTLS transmission and official SEFIN homologation. |
| **`rullst-connect`** | Social login / OAuth2 / OIDC providers (Google, Apple, GitHub, Discord, Auth0, Cognito) with PKCE and rotating JWKS. | 🟢 **`[Implemented / Bounded]`**: OAuth2/OIDC clients with constant-time PKCE comparison, validated discovery, bounded JWKS refresh/cache policy, and deterministic mock credentials.<br/>🔵 **`[Roadmap]`**: Message brokers belong in a future messaging boundary rather than being implied by this OAuth-focused crate. |
| **`rullst-iot`** | `no_std` sensor telemetry models and an Ed25519-signed firmware-manifest verification gate. | 🟢 **`[Implemented / Bounded]`**: Ed25519 manifest verification with in-process anti-rollback state, target/hash/length checks, and `no_std` telemetry frames.<br/>🟡 **`[Simulador Dev]`**: In-memory GPIO/I2C/BLE mocks under `feature = "experimental-simulators"`.<br/>🔵 **`[Roadmap]`**: Persistent counter/boot integration, firmware download/flashing, MQTT 5 transport, and hardware HSM drivers. |
| **`rullst-mail`** | Multi-transport transactional email engine (Resend, SendGrid, Postmark, AWS SES, SMTP) with deliverability and outbound-data controls. | 🟢 **`[Implemented / Bounded]`**: Named transports, anti-CRLF validation, disposable-domain filtering, tenant-aware pipeline, and expiring HMAC tracking tokens.<br/>🟡 **`[Offline Mock]`**: Memory/Log mock transports. |
| **`rullst-studio`** | Local Developer Control Room (`http://127.0.0.1:5555`), clean route navigation, live system telemetry visualizers. | 🟢 **`[Implemented / Bounded]`**: Local control center, `RadarSnapshot` telemetry, database/migration surfaces when configured, and explicit `Unavailable` states for unconnected probes. |
| **`rullst-nexus`** | Auto-generated Admin CMS (`/nexus`), dynamic model CRUD, AI Admin Assistant (`/nexus/chat`), SOC Threat Radar. | 🟢 **`[Implemented / Bounded]`**: Fail-closed construction, required authentication policy, admin role layer, server-side field policy, bounded batch operations, and escaped/DOM-safe rendering for audited paths. Host identity and tenant ownership remain application contracts. |
| **`rullst-macros`** | Procedural macros (`html!`, `rullst::model`, `rullst::runtime::main`) and compatibility helpers. | 🟢 **`[Implemented / Bounded]`**: Compile-time `html!` escaping with explicit `RawHtml`, model/runtime macros, and `trybuild` diagnostics.<br/>🟠 **`[Partial]`**: `server_function` preserves typed signatures, but browser argument transport and matching server-side RPC registration are not end-to-end. |
| **`cargo-rullst`** | Developer CLI toolkit, scaffolding generators (`make:*`), project blueprints, AST IDOR static route scanner. | 🟢 **`[Implemented / Bounded]`**: Interactive wizard, generators, heuristic IDOR scanner, CycloneDX exporter, and doctor diagnostics.<br/>🟠 **`[Partial]`**: A complete compiling matrix for every generator/blueprint/feature combination is still release-engineering work. |

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
* **Bounded Heuristic Inspector:** ASCII case-insensitive signature matching covers selected SQL injection, traversal, SSRF, shell/JNDI patterns across URI, non-secret headers, and supported bounded textual/JSON bodies. Percent decoding and body/JSON inspection allocate; this control does not replace typed parsing, SQL binds, validation, authorization, or SSRF allowlists.
* **Login Guard Tarpit:** Returns progressive delay decisions (0s to 5s) for the caller to apply and maintains bounded, temporary in-memory jails for repeated failures keyed by a hashed identity.

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
