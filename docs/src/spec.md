<!-- “Antes de gerar qualquer coisa, leia e siga estritamente o arquivo da verdade abaixo. spec.md”  -->

# Rullst Specification 📄
### *"The Single Source of Truth (SST) for Framework Architecture & Conventions"*

This document is the **Single Source of Truth (SST)** for the **Rullst Framework**. It specifies the exact conventions, API structures, naming rules, and directory standards of Rullst.

> [!IMPORTANT]
> **AI Alignment Instruction:**
> Whenever updating, refactoring, or generating documentation and code for Rullst, **always** refer to this specification as the baseline. Do not invent or assume conventions outside of this document.

---

## 📂 1. Directory Structure Conventions

A standard Rullst application scaffold must strictly follow this folder hierarchy:

```text
my-app/
├── src/
│   ├── controllers/      # Route controllers (async modules)
│   │   └── mod.rs
│   ├── models/           # Active Record Models (rullst-orm entities)
│   │   └── mod.rs
│   ├── pages/            # Shared static HTML elements or full page layouts
│   │   └── mod.rs
│   └── main.rs           # Entrypoint, DB initialization, and Central routing
├── Cargo.toml            # Project cargo dependencies
└── Rullst.toml           # Framework configuration (databases, environment, etc.)
```

---

## 🛠️ 2. Naming Conventions

To guarantee consistency, both humans and AI coders must adhere to the following name normalization rules handled by the `cargo-rullst` generator:

* **File Names:** Standard Rust `snake_case` (e.g. `users_controller.rs`, `post_model.rs`).
* **Struct / Model / Documentation Names:** Standard `PascalCase` (e.g. `UsersController`, `PostModel`).
* **URL Paths:** Lowercase kebab-case (e.g. `/users`, `/user-profiles`).

---

## ⚡ 3. Core API Specifications

`rullst-core` is runtime-only by default. Database integration is selected with
the independent `orm` and `queue-sqlite` features. Crates such as Studio and
Nexus must request the features they use explicitly; the `rullst` application
umbrella enables both in its default feature set.

### 3.1. Server & Routing (`rullst::routing`)

* **Routing Macro:** central routing declared via the `routes!` macro, wrapping Axum routing handlers.
  ```rust
  let router = routes![
      get("/" => home),
      post("/posts" => posts_controller::store),
  ];
  ```
* **Server Lifecycle:**
  ```rust
  Server::new(router: Router)
      .run(port: u16) -> Result<(), Box<dyn std::error::Error>>
  ```

### 3.2. Server-Side Rendering (`rullst::macros`)

* **Macro:** `html!` procedural macro compiles HTML trees directly into static memory string concat builders.
* **XSS Protection:** Automatic HTML escaping on all dynamic variables wrapped in `{expr}`.
* **Raw Unescaped HTML:** Explicitly bypassed using the wrapper `rullst::html::RawHtml(String)`.
* **Lists/Iterators:**
  ```rust
  let mut list_builder = String::new();
  for item in items {
      list_builder.push_str(&html! { <li>{item}</li> });
  }
  html! { <ul>{ rullst::html::RawHtml(list_builder) }</ul> }
  ```

### 3.3. Active Record ORM (`rullst-orm`)

* **Model definition:**
  ```rust
  #[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
  #[orm(table = "table_name")]
  pub struct Model {
      pub id: i32,
      // ... fields
  }
  ```
* **Static queries:**
  * `Model::all().await` -> `Result<Vec<Model>, sqlx::Error>`
  * `Model::find(id).await` -> `Result<Model, sqlx::Error>`
* **Instance Operations:**
  * `let mut instance = Model { ... };`
  * `instance.save().await` -> `Result<(), sqlx::Error>` (handles auto-incrementing inserts or updates).
  * `instance.delete().await` -> `Result<(), sqlx::Error>`

---

## 💻 4. CLI Specifications (`cargo-rullst`)

* **Project Creation:**
  `cargo rullst new <name>`
  * *Convention:* Automatically extracts the package name from path expressions (e.g., `..\dummy_test` -> `dummy_test`).
* **Controller/Island Scaffolding:**
  `cargo rullst make:controller <Name>`
  `cargo rullst make:island <Name>`
  * *Behavior:* Generates `src/controllers/<snake_name>_controller.rs` with `index` and `show` actions. Appends declaration to `src/controllers/mod.rs`. Adds `pub mod controllers;` to the top of `src/main.rs`.
* **Documentation SSG (RullstPress):**
  `cargo rullst docs build` and `cargo rullst docs dev`
  * *Behavior:* Compiles markdown files in `docs/` into a static site inside `docs/dist/`.

---

## 🧱 5. Controller Architecture

Controllers handle business logic and HTTP responses.
* **Module Structure**: Each controller is a separate module inside `src/controllers/` (e.g., `users_controller.rs`).
* **Function Signatures**: Functions must be asynchronous and return a type that implements `axum::response::IntoResponse` (or `Result<impl IntoResponse, AppError>`).
* **Database Access**: Controllers must **never** contain raw `sqlx::query!` macros inline. Database logic must be delegated to the Active Record ORM methods (`.save()`, `.all()`, etc.) or encapsulated within specific `impl Model` functions.
* **Standard Actions:** 
  * `pub async fn index()`: List all resources.
  * `pub async fn show(Path(id): Path<i32>)`: Show a specific resource.
  * `pub async fn store(Form(payload): Form<CreateDto>)`: Create a new resource.
  * `pub async fn update(Path(id): Path<i32>, Form(payload): Form<UpdateDto>)`: Update a resource.
  * `pub async fn delete(Path(id): Path<i32>)`: Delete a resource.

---

## 📄 6. HTML Pages & Components

Rullst uses a functional approach for HTML rendering, relying on the `html!` macro.
* **Organization:** Pages and components reside in `src/pages/`.
* **Functional Components:** Pages and components are simply Rust functions. They are not structs or classes.
* **Props/Data:** Pass data into pages and components as regular function arguments.
* **Return Type:** Components should return a `String` (or `rullst::html::RawHtml`) so they can be embedded in other `html!` calls. Route-level pages should return `axum::response::Html<String>` to be served directly.
* **Example:**
  ```rust
  pub fn button_component(label: &str, url: &str) -> String {
      html! { <a href={url} class="btn">{label}</a> }
  }
  
  pub fn home_page(user_name: &str) -> axum::response::Html<String> {
      let content = html! {
          <div>
              <h1>"Welcome, "{user_name}</h1>
              { rullst::html::RawHtml(button_component("Click Me", "/click")) }
          </div>
      };
      axum::response::Html(content)
  }
  ```

---

## 🚨 7. Error Handling

Consistent error handling ensures safety and predictable API responses.
* **Default Error Type:** The framework expects typed domain error enums (`AppError`, `CapitalError`, `OrmError`, `FiscalError`, etc.) deriving `thiserror::Error`.
* **Implementation:** `AppError` must implement `axum::response::IntoResponse`.
* **Zero-Panic Rule:** Non-test code must never call `unwrap()`, `expect()`, or `panic!()`.
* **HTTP Codes:** The `IntoResponse` implementation maps internal errors to appropriate HTTP status codes (e.g., `404 Not Found`, `500 Internal Server Error`).

---

## 🛡️ 8. Middlewares

Middlewares intercept requests for authentication, logging, security, etc.
* **Location:** Middlewares are placed in `src/middlewares/`.
* **Standard Signature:** Following Axum's `from_fn` pattern, a middleware function looks like:
  ```rust
  use axum::{extract::Request, middleware::Next, response::Response};
  
  pub async fn my_middleware(req: Request, next: Next) -> Response {
      let response = next.run(req).await;
      response
  }
  ```
* **Registration:** Middlewares are registered on the router using Axum's `.layer()` or through Rullst's server configuration wrapper.

---

## 🛡️ 9. Architectural Guidelines for Backward Compatibility

To guarantee stress-free updates for Rullst users, all framework code must adhere to these backward compatibility rules:

### 9.1. The Builder Pattern and `#[non_exhaustive]`
Any public configuration struct or extensible enum exposed by the framework **must** use the `#[non_exhaustive]` attribute. This prevents direct struct instantiation, ensuring that adding new fields in future minor versions will not break user code.
* **Mandatory Usage:** All instantiation must be done via a constructor (`new()`) and the Builder Pattern (`with_...()`).

### 9.2. Deprecation Lifecycle (`#[deprecated]`)
The framework will never abruptly remove or rename a public function, struct, or method. If a breaking change is required, the old API must be kept alive for at least one minor version with `#[deprecated]`.

### 9.3. Sealed Traits
If the framework exposes a Trait that is meant to be used by the user but **not implemented** by the user, it must use the "Sealed Trait" pattern.

---

## 🏗️ 10. CLI Modular Architecture (`cargo-rullst`)

The `cargo-rullst` CLI must **never** be allowed to grow into a monolithic `main.rs`. Once the file exceeds ~1000 lines, refactoring into the module structure below is mandatory:

```text
cargo-rullst/
├── src/
│   ├── main.rs               # ≤ 80 lines: Entry point only. Dispatches to cli or ui.
│   ├── cli.rs                # Clap structs, Commands enum, argument definitions.
│   ├── ui/                   # Everything visual: banners, spinners, menus, boxes.
│   ├── generators/           # Scaffold logic: writes files to disk on user's project.
│   └── blueprints/           # Blueprint template definitions.
```

### 10.1. The `main.rs` Purity Rule
`main.rs` contains only:
1. Crate-level attributes.
2. Module declarations.
3. `fn main()` dispatching to `cli` or `ui`. Zero business logic, zero file I/O.

### 10.2. Template String Rules
Never embed multi-line templates inside generator functions. Use `include_str!()` or typed constants inside `blueprints/` with `r###"..."###` triple-hash raw string literals.

---

## 🎨 11. Rullst Blueprints Engine — Design Rules

| Blueprint ID | Name | Description |
|---|---|---|
| `0` | 📝 Blank Starter | Minimal HTMX reactive counter. Clean baseline. |
| `1` | 🎓 LMS / Course Platform | Courses + Lessons models, migrations with seed data, glassmorphic video player. |
| `2` | 🛍️ SaaS Starter | Auth system + Stripe pricing panels + user dashboard. |
| `3` | 📰 Blog / Content System | Post model, auto-CMS via Nexus, glassmorphic press feed. |

---

## 🔐 12. Environment Variables & Third-Party Secrets

Any scaffold integrating third-party services generates a `.env` with commented placeholder values, automatically protected via `.gitignore`. An accompanying `.env.example` is committed for onboarding.

---

## 💳 13. Multi-Provider Billing & National Fiscal Engine (`rullst-capital`)

`rullst-capital` provides unified subscription management, multi-gateway payments, contractor payouts, and a contained offline preview of NFS-e Nacional data structures. Live fiscal authorization is roadmap work and must fail closed.

### 13.1. Provider Architecture
All billing integrations implement one or more decoupled asynchronous traits:
* `PaymentProvider`: Checkout sessions, payment intent creation, customer portal URLs.
* `SubscriptionProvider`: Recurring plan sync, cancellation, upgrade, status querying.
* `PayoutProvider`: Multi-currency contractor payouts, recipient verification, transfer tracking.

### 13.2. Supported Gateways & Mock Fallback Standard
Provider adapters include **Stripe, LemonSqueezy, MercadoPago, InfinitePay, PicPay, Razorpay, Polar, Paddle, Wise, Coinbase Commerce, and Alipay**. Each adapter documents its implemented live capabilities; an adapter's presence is not a claim that every payment, subscription, payout, portal, or webhook operation is available.
* **Mock Invariant:** All constructors (`new(impl Into<String>, ...)`) must seamlessly fall back to an offline deterministic mock sandbox when initialized with empty or `mock_*` credentials.

### 13.3. NFS-e Nacional Contained Preview (`FiscalEngine`)
* **Offline DPS Fixture:** Builds escaped DPS-shaped XML for deterministic local tests. It is not validated against official XSDs and is not an authorized invoice.
* **Explicit Mock Result:** Only `NfseEnvironment::Mock` executes and returns `FiscalResponseKind::OfflineMock` with a non-authorized identifier.
* **Fail-Closed Live Modes:** `Homologation` and `Production` return a typed `Unsupported` error until PKCS#12 handling, XML C14N/XMLDSig, XSD validation, mTLS, strict response parsing, and official end-to-end homologation are implemented and independently verified.

---

## 🛡️ 14. Enterprise Security, RASP & Zero-Trust (`rullst-security`)

`rullst-security` delivers defense-in-depth security layers designed to protect applications without requiring external proxy dependencies:

### 14.1. Mandatory Middleware Layers
* `SecureHeadersLayer`: Applies a strict security-header baseline with per-response CSP nonces, HSTS, frame restrictions, and Permissions-Policy. External scanners remain environment-dependent; the framework does not guarantee a particular third-party score.
* `WafMiddleware`: Deep-inspects inbound URI query strings and request bodies for SQL Injection (SQLi), Cross-Site Scripting (XSS), Path Traversal, and Command Injection.
* `LoginJailLayer`: Tarpit rate-limiting middleware that exponentially delays brute-force authentication attacks on login routes.
* `DlpInterceptor`: Data Loss Prevention engine that masks credit card numbers, CPF/CNPJ documents, and private API keys from outgoing HTTP responses and structured logs.
* `HoneypotTrap`: Injects invisible honeypot form fields to immediately ban and record automated scraping bots.

### 14.2. Cryptographic Security Standards
* **Constant-Time Verification:** Webhook signatures and security tokens must use `subtle::ConstantTimeEq` or cryptographic HMAC verification to completely eliminate timing side-channel attacks.
* **TOTP MFA (RFC-6238):** Hardware-token compatible multi-factor authentication with Base32 secret generation and QR codes.
* **Subresource Integrity (SRI):** Automatic SHA-384 / SHA-512 SRI hash injection on external static script and style tags.

---

## 🤖 15. AI Agent & LLM Orchestration (`rullst-ai`)

`rullst-ai` provides a provider-agnostic client with explicit provider capability boundaries and deterministic offline fixtures:

### 15.1. Universal Provider Interface
Supports **Google Gemini, OpenAI, Anthropic Claude, DeepSeek, and Ollama** through the guarded `AiClient` application interface and the lower-level `AiProvider` trait.

### 15.2. Safety & Guardrails
* **Prompt Injection Filter:** Real-time token heuristics detecting jailbreak and system-prompt leak attempts.
* **PII Redaction:** Automatically scrubs personal identifiable information before outbound LLM transmission.
* **Structured Outputs:** Separates parseable JSON mode from explicit JSON Schema output. Native schema requests fail with `UnsupportedCapability` when a provider cannot enforce them.

---

## 📊 16. Studio Control Room & Nexus Admin CMS (`rullst-studio` & `rullst-nexus`)

### 16.1. Rullst Studio (`/studio`)
* Local developer control room running at `http://127.0.0.1:5555`.
* **Telemetry Probes:** Direct, live telemetry inspection via `RadarSnapshot::collect()` and `SpanCollector`. Hardcoded mock data in Studio is strictly prohibited.
* **Clean URLs:** Standardized clean paths without legacy subpaths (e.g. `/studio/radar`, `/studio/capital`, `/studio/security`, `/studio/traces`).
* **Design System:** Unified dark glassmorphic `studio_layout` with live status pulse badges and zero client-side build steps.

### 16.2. Rullst Nexus (`/nexus`)
* Auto-generated administrative interface with dynamic entity CRUD and an embedded AI Admin Assistant (`/nexus/chat`). Nexus is fail-closed without an explicit authenticated access policy, applies an administrator role gate to every route, and enforces hidden/read-only field policy in handlers as well as views.

---

## 📡 17. Edge Sensor Protocols & Message Queues (`rullst-iot` & `rullst-connect`)

* **`rullst-iot`**: `no_std` telemetry models, protocol frame helpers, and strict Ed25519-signed firmware manifest verification. MQTT transport, hardware HSM backends, firmware flashing/bootloader control, and post-quantum cryptography remain roadmap work; deterministic `Simulated*` fixtures require the explicit `experimental-simulators` feature and carry no security or protocol guarantee.
* **`rullst-connect`**: OAuth2/OIDC and social-login provider adapters with strict redirect/discovery validation, deterministic offline credentials, and rotating JWKS caches. Queue transports and application WebSockets/SSE currently live in `rullst-core`; Kafka, RabbitMQ, and Redis Streams adapters in Connect remain roadmap work.

---

## ✉️ 18. Transactional Mail Engine (`rullst-mail`)

* **Multi-Transport Engine**: Native support for **Resend, SendGrid, Postmark, and SMTP** with seamless offline test mocks.
* **Background Queue Integration**: Asynchronous, non-blocking email delivery integrated with Rullst background task workers.
