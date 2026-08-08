<p align="center">
  <img src="https://raw.githubusercontent.com/Rullst/Rullst/main/Rullst.png" alt="Rullst Logo" width="300">
</p>

<h1 align="center">Rullst 📜🦀🌐🚀</h1>
<h3 align="center"><i>Rust for those who want to build securely and easily, but not suffer.
</i></h3>

<p align="center">
  <a href="https://crates.io/crates/rullst"><img src="https://img.shields.io/crates/v/rullst?style=for-the-badge&color=10b981&logo=rust" alt="Crates.io"></a>
  <a href="https://crates.io/crates/rullst"><img src="https://img.shields.io/crates/d/rullst?style=for-the-badge&color=blue" alt="Crates.io Downloads"></a>
  <a href="https://docs.rs/rullst"><img src="https://img.shields.io/docsrs/rullst?style=for-the-badge&logo=docsdotrs" alt="Docs.rs"></a>
  <a href="https://github.com/Rullst/Rullst/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/ci.yml?style=for-the-badge&label=Build" alt="Rust CI"></a>
  <img src="https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge" alt="License: MIT">
</p>

<br/>

**Rullst** is an opinionated, developer-first full-stack web framework for Rust, obsessively designed for **Emotional Productivity and Security**. It solves the biggest problem in the Rust web ecosystem: the high barrier of entry. With Rullst, you spend your energy building your business, not fighting borrow checkers and manual routing setups.

---

### 📚 Documentation & Community

We've rewritten our entire documentation from scratch into a beautiful, high-performance website. Discover everything Rullst can do, read the benchmarks, and master the framework:

👉 **[Explore the Official Website & Docs](https://rullst.github.io)**

💬 **[Join the Community on Discord](https://discord.gg/2ntKFtsSjw)**

> **Found a bug?** [Report an Issue](https://github.com/Rullst/Rullst/issues)

---

### 🏛️ The Rullst Monorepo (v12.0.0+)

Rullst is now a unified Monorepo! The framework's core (`rullst`), the database layer (`rullst-orm`), and the frontend connectivity (`rullst-connect`) are now engineered in lockstep under a single repository. This unified architecture ensures 100% compatibility across the stack, centralized security audits, and a seamless developer experience from backend to edge.

**Explore the Ecosystem:**
- 🦀 **[Rullst Core (Web Framework)](https://github.com/Rullst/Rullst)**
- 💾 **[Rullst-ORM (Database Layer)](https://github.com/Rullst/Rullst/tree/main/rullst-orm)**
- 🔌 **[Rullst-Connect (Frontend Integration)](https://github.com/Rullst/Rullst/tree/main/rullst-connect)**

---

### 🔓 Zero Lock-In Guarantee (100% Axum & SQLx)

Rullst is built directly on top of **Axum**, **Tokio**, and **Tower**. It does not invent proprietary HTTP abstractions or locked-in router types. Every Rullst controller, extractor, and middleware maps 1:1 to standard Axum and Tower equivalents:

- **Incremental Adoption:** Mount existing `axum::Router` instances directly into `rullst::server::Server`.
- **Standard SQLx:** Run raw `sqlx::Pool` queries alongside `rullst-orm` without wrappers.
- **Escape Hatch:** Convert Rullst controllers back to raw Axum with a 1-line import change.
- 📖 Read the full [Axum & SQLx Migration & Escape Hatch Guide](https://github.com/Rullst/Rullst/blob/main/docs/src/axum-sqlx-migration.md).

---

<h3 align="center">🛡️ Enterprise-Grade Security</h3>

<p align="center">
  Rullst is built with a "Zero-Panic Policy" and tested against the most rigorous standards in the industry.<br/>
  Our continuous pipeline guarantees absolute safety for production edge infrastructure:
</p>


<div align="center">

| Security Audit | Status | Description |
| :--- | :---: | :--- |
| **OSSF Scorecard** | [![OSSF Scorecard](https://img.shields.io/ossf-scorecard/github.com/Rullst/Rullst?label=&style=flat-square)](https://github.com/Rullst/Rullst/actions/workflows/scorecards.yml) | Supply-chain security & best practices |
| **Codecov** | [![Codecov](https://img.shields.io/codecov/c/github/Rullst/Rullst?style=flat-square&label=)](https://codecov.io/gh/Rullst/Rullst) | Strict code coverage enforcement |
| **OpenSSF** | [![OpenSSF](https://img.shields.io/badge/status-passing-brightgreen?style=flat-square&label=)](https://www.bestpractices.dev/projects/13321) | Open source security standards |
| **Matrix DB Tests** | [![Matrix DB Tests](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/ci.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/ci.yml) | Dockerized PostgreSQL & MySQL integration tests |
| **Continuous Fuzzing** | [![Continuous Fuzzing](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/fuzzing.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/fuzzing.yml) | Fuzzing against edge cases & panics |
| **Property Testing** | [![Property Testing](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/proptest.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/proptest.yml) | Validating complex logic against edge cases |
| **CodeQL SAST** | [![CodeQL SAST](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/codeql.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/codeql.yml) | Advanced semantic code analysis |
| **OWASP ZAP DAST** | [![OWASP ZAP DAST](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/dast-zap.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/dast-zap.yml) | Dynamic vulnerability scanning |
| **Cargo Deny** | [![Cargo Deny](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/cargo-deny.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/cargo-deny.yml) | Banning unmaintained/vulnerable crates |
| **Cargo Audit** | [![Cargo Audit](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/audit.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/audit.yml) | Continuous scanning for crate vulnerabilities |
| **Cargo SemVer** | [![SemVer Checks](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/semver.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/semver.yml) | Strict SemVer API breakage checks |
| **Cargo Machete** | [![Cargo Machete](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/machete.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/machete.yml) | Detecting unused and bloated dependencies |
| **Benchmark CI** | [![Benchmark CI](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/bench.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/bench.yml) | Continuous performance regression testing |
| **Snapshot Testing** | [![Snapshot Testing](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/ci.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/ci.yml) | UI & Macro structural regression testing |
| **Spellcheck CI** | [![Spellcheck CI](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/spellcheck.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/spellcheck.yml) | Automated typo detection across docs and code |
| **Clippy Lints** | [![Clippy Lints](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/ci.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/ci.yml) | Strict compiler & style linting |
| **Unsafe Policy** | [![Unsafe Policy](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/unsafe-policy.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/unsafe-policy.yml) | 100% memory safe. No unsafe code blocks |
| **Miri UB Detection** | [![Miri](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/miri.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/miri.yml) | Detecting Undefined Behavior and memory leaks |
| **Kani Verifier** | [![Kani Verifier](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/kani.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/kani.yml) | Automated reasoning and formal verification |
| **Mutation Testing** | [![Mutation Testing](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/mutants.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/mutants.yml) | Mutation testing for test suite robustness |
| **Architecture Linter** | [![TangleGuard](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/tangleguard.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/tangleguard.yml) | Enforcing architectural boundaries |
| **SLSA Level 3** | [![SLSA 3](https://img.shields.io/badge/SLSA-Level_3-brightgreen?style=flat-square&label=)](https://slsa.dev/) | Supply-chain Levels for Software Artifacts |
| **Panic Policy** | [![Zero Panics Policy](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/zero-panics.yml?style=flat-square&label=Zero%20Panics)](https://github.com/Rullst/Rullst/actions/workflows/zero-panics.yml) | Graceful error handling across the framework |
| **Secret Scanning** | [![Trufflehog](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/trufflehog.yml?style=flat-square&label=Trufflehog)](https://github.com/Rullst/Rullst/actions/workflows/trufflehog.yml) | Automated CI prevention of leaked credentials |
| **no_std Build Check** | [![no_std Build](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/no_std-build.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/no_std-build.yml) | Validates `rullst-iot` compiles on STM32, ESP32-C3, Cortex-M bare-metal targets |
| **OTA Signature Verification** | [![OTA Integrity](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/iot-integration.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/iot-integration.yml) | Ed25519 cryptographic integrity check on all OTA firmware updates |
| **PQC Compliance Audit** | [![PQC Compliance](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/pqc-compliance.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/pqc-compliance.yml) | Weekly NIST ML-KEM / Kyber & HSM compliance audit (unsafe-free cryptographic modules) |
| **Concurrency Sanitizers** | [![Sanitizers](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/sanitizers.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/sanitizers.yml) | ThreadSanitizer & AddressSanitizer race-condition and heap validation |
| **E2E Smoke Tests** | [![E2E Smoke Tests](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/e2e-smoke.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/e2e-smoke.yml) | Automated end-to-end HTTP, SSR, SQLite, and Security Header verification |
| **MSRV** | [![MSRV](https://img.shields.io/badge/MSRV-1.96.0-orange?style=flat-square&label=)](https://github.com/Rullst/Rullst) | Minimum Supported Rust Version |

</div>

> 📖 **[Read the detailed breakdown of all our CI/CD Security Workflows here](https://github.com/Rullst/Rullst/blob/main/WORKFLOWS.md)**

<br>
<h2 align="center"> CLI ⚡ Rullst Framework ⚡ </h2>
<p align="center">
  <img src="https://github.com/Rullst/Rullst/blob/main/gifs/gif.gif" alt="Rullst CLI Initiating LMS Blueprint" width="100%"/>
</p>

<h2 align="center">Click to Watch: How to build a SaaS Blueprint with Rullst </h2>
<p align="center">
<a href="https://www.youtube.com/watch?v=nDXLeNM327g">
  <img src="https://img.youtube.com/vi/nDXLeNM327g/hqdefault.jpg" alt="How to build a SaaS with Rullst" width="430" />
</a>
</p>

<table align="center" width="100%">
  <tr>
    <th align="center" width="50%"><h2>SaaS Blueprint</h2></th>
    <th align="center" width="50%"><h2>LMS Blueprint</h2></th>
  </tr>
  <tr>
    <td align="center">
      <img src="https://github.com/Rullst/Rullst/blob/main/gifs/gif1.gif" alt="SaaS Blueprint" width="100%" />
    </td>
    <td align="center">
      <img src="https://github.com/Rullst/Rullst/blob/main/gifs/gif2.gif" alt="LMS Blueprint" width="100%" />
    </td>
  </tr>
</table>


---

### ⚡ Unmatched Performance

Rullst's "Zero-Cost Abstraction" architecture provides full-stack productivity without sacrificing bare-metal speed:

- **SSR HTML5 Rendering**: Zero-bundle static string rendering avoiding Virtual DOM allocations.
- **Macro Routing (`routes!`)**: Direct compile-time static dispatch powered by Axum and Tokio.
- **HtmlSanitizer & XSS Shield**: High-speed Ammonia AST payload filtering.
- **RbacGuard Role & Ownership BOLA**: Zero-allocation bitflags security authorization.
- **Vault In-Memory Zeroization**: Cryptographic drop memory wiping preventing cold-boot RAM inspection.
- **Stripe Webhook Signature Verification**: Constant-time HMAC-SHA256 protecting against timing attacks.
- **Passkey WebAuthn Challenge Parser**: High-performance FIDO2 passwordless auth parsing.
- **Zero-Trust Device Fingerprinting**: Subnet-aware session binding with zero runtime overhead.
- **AI Guardrail Prompt Sanitizer**: In-memory prompt injection neutralization before LLM transport.
- **RAG Cosine Vector Similarity**: SIMD-accelerated local vector embedding similarity computation.

> 📊 **Explore live results & reproducible suites:**
> - [**📈 Interactive Benches Dashboard**](https://rullst.github.io/Rullst/#benches) — Real-time telemetry and microsecond visualizers on the official site.
> - [**⚖️ Comparative Benchmarks**](https://github.com/Rullst/Benchmarks) — Open benchmark repository with reproducible TechEmpower-style setups, Criterion HTML reports, memory profiling, and continuous performance regression CI.

- 🚀 **Hybrid Hot-Reloading & Fast Linkers**: Sub-second incremental compilation with `mold` and `lld` pre-configured in `.cargo/config.toml`, paired with WebSockets morphdom UI hot-swapping.
- 🎨 **Developer Control Room & Nexus CMS**: An all-in-one Web Suite (`cargo rullst studio` at `:5555`) with Data Browser, Visual Threat Radar, Real-time Metrics, and auto-generated Admin Panels (`/nexus`) from your Structs.
- 🛡️ **RASP Engine & Pre-Controller Shield**: Kernel-level AST payload filtering protecting against XSS, SQLi, and BOLA before requests ever reach your controllers.
- 🔑 **Passkeys & WebAuthn (FIDO2)**: Hardware-backed passwordless authentication using biometric face/touch ID and security keys.
- 🌐 **Provider-Agnostic AI (Local & Cloud)**: Connect to **ANY local LLM** (Ollama, LM Studio, vLLM, LocalAI, Jan.ai) or commercial cloud (Gemini, Claude, OpenAI, DeepSeek) with built-in prompt injection filters and PII masking.
- 💳 **Capital SaaS Ledger & Billing**: Real-time MRR/ARR analytics and constant-time HMAC signature verification for Stripe and LemonSqueezy webhooks.
- 🔐 **Enterprise Vault & Memory Zeroization**: Field-level database encryption (`#[orm(encrypted)]`) with cryptographic `Zeroize` memory clearing upon drop.
- 🔄 **Expressive Active Record Transactions**: Borrow-checker safe `User::transaction(|tx| async move { ... })` with automatic task-local scoping (`CURRENT_TX`), commit-on-success, and rollback-on-error behavior.
- 🔄 **Reverse ORM Scaffolding**: Automatically reverse-engineer Rust `struct` models from existing database tables using `cargo rullst make:models-from-db`.
- 🔍 **Static CLI Inspection**: Inspect active route tables, ORM models, and JSON schemas directly in the terminal via `cargo rullst inspect`.
- 🛡️ **Zero-Panic Policy**: Hardened architecture built with typed `AppError` enums for 100% crash-free edge infrastructure.
- ⚡ **Interactive Scaffolding**: 1-click generators for Auth, ERPs, SaaS Starters, Uptime Monitors, and Cloud Deployments (`cargo rullst deploy`).

---
<br>

![Rullst CLI Initiating LMS Blueprint](https://github.com/Rullst/Rullst/blob/main/Interative%20Terminal%20Dashboard.png)

---

### 💻 The Beauty of Rullst

```rust
use rullst::{routing::get, html, Server, Response};

#[routes]
fn home() -> Response {
    html! {
        <div class="h-screen bg-slate-900 text-emerald-400 flex items-center justify-center">
            <h1>"Hello, Rullst!"</h1>
        </div>
    }
}

#[tokio::main]
async fn main() {
    Server::new()
        .route("/", get(home))
        .run()
        .await;
}
```

---

<h2 align="center">🥊 Rullst vs The Ecosystem (Honest Comparison)</h2>

<p align="center">
Rust has a breathtaking ecosystem, but finding the right tool can be overwhelming.<br>
Here is an honest, objective breakdown of where Rullst stands compared to other beloved frameworks.
</p>

### 🔬 HTTP & API Frameworks (Actix-Web, Axum, Salvo, Poem)
These are the **titans of the Rust web ecosystem**. They provide pristine routing, middlewares, and blazing-fast HTTP primitives. Actix-Web and Rocket pioneered the space, while Axum, Salvo, and Poem brought new paradigms.
* **The Catch:** They are fundamentally focused on HTTP. You have to wire the rest of the application yourself. You must choose, configure, and integrate your own Database ORM, Auth logic, Webhooks, CLI, and Background Workers.
* **Where Rullst Excels:** **Batteries Included.** Rullst actually uses *Axum* under the hood for its HTTP routing! But instead of leaving you in an empty room, Rullst gives you a fully furnished house. You get a CLI, ORM, Auth, Stripe integration, Background Workers, and **automatic OpenAPI & TypeScript SDK generation** out-of-the-box in 1 minute.

### 🚂 Full-Stack Frameworks (Loco, Topcoat)
**Loco** is a fantastic full-stack framework heavily inspired by Rails. It also uses Axum and provides great generators.
**Topcoat** is an experimental, batteries-included framework from the Tokio team that focuses on reactive server-side rendering (SSR) without writing JavaScript.
* **Where Rullst Excels:** **Emotional Productivity & DX.** Rullst takes a radically opinionated stance on Developer Experience. We provide an immersive Web-based Database Studio (`cargo rullst studio`), built-in Wasm Islands, zero-panic architectural guarantees, Nix reproducibility, and native Omni (Desktop/Mobile via Tauri) scaffolding. If you want the absolute easiest, most visually pleasing DX in Rust, Rullst is your home.

### 🎨 Isomorphic Full-Stack Frameworks (Dioxus, Leptos)
These are cutting-edge frameworks that let you write both frontend and backend in a single Rust file using Server Functions and SSR (similar to Next.js or Nuxt).
* **The Catch:** They are heavily **Frontend/Component-Driven**. Your server's primary job is to hydrate and serve UI components. If you need a traditional backend architecture (dedicated Workers, Stripe webhooks, robust ORM migrations, pure REST APIs for mobile apps), an isomorphic model can sometimes feel restrictive or overly coupled to the UI.
* **Where Rullst Excels:** **Architectural Freedom & Synergy.** Rullst is an **API-First / Traditional Full-Stack** (like Rails or Laravel). It gives you an uncompromised, heavy-duty backend layer. But we don't compete with Dioxus/Leptos/Tauri—we *embrace* them! Rullst allows you to use Dioxus for your frontend natively via Wasm Islands (`cargo rullst build:client`), or package your entire application into Desktop & Mobile apps via **Tauri** (`cargo rullst make:omni`).

### 📊 The Full-Stack Feature Matrix

| Feature | **Rullst** | **Loco** | **Topcoat** | **Dioxus / Leptos** | **Axum / Actix** |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **HTTP & High-Performance Routing** | ✅ (Axum Engine) | ✅ | ✅ | ✅ (SSR) | ✅ |
| **Active Record & Data Mapper ORM** | ✅ (`rullst-orm`) | ✅ (SeaORM) | ✅ (Toasty) | ❌ | ❌ |
| **Compile-Time Zero-Cost DI Container** | ✅ (`rullst::di` & `Inject<T>`) | ❌ | ❌ | ❌ | ❌ |
| **1-Click PaaS Cloud Deployment** | ✅ (`cargo rullst deploy`) | ❌ | ❌ | ❌ | ❌ |
| **RASP Security Layer (Pre-Controller Inspection)** | ✅ (`rullst-security`) | ❌ | ❌ | ❌ | ❌ |
| **Passkeys & WebAuthn (FIDO2 Passwordless)** | ✅ (`rullst-auth::passkey`) | ❌ | ❌ | ❌ | ❌ |
| **Granular RBAC & Role Permission Matrix** | ✅ (`rullst-auth::rbac`) | ❌ | ❌ | ❌ | ❌ |
| **Zero-Trust Device & Session Fingerprinting** | ✅ (`rullst-security::zero_trust`) | ❌ | ❌ | ❌ | ❌ |
| **Rullst Vault & Transparent Field Encryption** | ✅ (`#[orm(encrypted)]` + `Zeroize`) | ❌ | ❌ | ❌ | ❌ |
| **Synthetic Honeypots & Automated Bot Ban** | ✅ (`rullst-honey`) | ❌ | ❌ | ❌ | ❌ |
| **HMAC Tamper-Proof Cryptographic Audit Log** | ✅ (`rullst-audit-log`) | ❌ | ❌ | ❌ | ❌ |
| **Visual Threat Radar (SOC Dashboard)** | ✅ (`/studio/security`) | ❌ | ❌ | ❌ | ❌ |
| **Air-Gapped Local & Multi-Cloud AI (Zero-Leak)** | ✅ (`rullst-ai`: Ollama, LM Studio, vLLM, OpenAI, Claude, Gemini, DeepSeek) | ❌ | ❌ | ❌ | ❌ |
| **LiveView Server-Driven Reactive UI** | ✅ (`rullst::live` + `make:live`) | ❌ | ✅ (Signals) | ❌ | ❌ |
| **gRPC Microservices & Protobuf Scaffolding** | ✅ (`rullst-grpc` / Tonic) | ❌ | ❌ | ❌ | ❌ |
| **Kubernetes Native Manifests & Health Probes** | ✅ (`make:k8s` + `/health`) | ❌ | ❌ | ❌ | ❌ |
| **Interactive Scalar API Docs Playground** | ✅ (Built-in `/docs`) | ❌ | ❌ | ❌ | ❌ |
| **Web-based Database Studio** | ✅ (Rullst Studio) | ❌ | ❌ | ❌ | ❌ |
| **Auto-Generated Admin Panel (CMS)** | ✅ (Rullst Nexus) | ❌ | ❌ | ❌ | ❌ |
| **Kernel Telemetry & Prometheus Exporter** | ✅ (`rullst::radar` + `/metrics`) | ❌ | ❌ | ❌ | ❌ |
| **Embedded IoT & Edge Hardware (`#![no_std]`)** | ✅ (`rullst-iot` / STM32 / ESP32) | ❌ | ❌ | ❌ | ❌ |
| **SaaS Revenue Dashboard & Stripe Billing** | ✅ (`rullst-capital`) | ❌ | ❌ | ❌ | ❌ |
| **Background Workers & Redis Task Queues** | ✅ (`rullst::queue`) | ✅ (Task worker) | ❌ | ❌ | ❌ |
| **Wasm Islands & Hybrid SSR** | ✅ (`#[client_component]`) | ❌ | ❌ | ✅ (Core focus) | ❌ |
| **TypeScript AST SDK Generator** | ✅ (`cargo rullst generate:ts`) | ❌ | ❌ | ❌ | ❌ |
| **Zero-Panics Policy Enforced** | ✅ (Typed `AppError` & Lints) | ❌ | ❌ | ❌ | ❌ |
| **Framework Escape Hatch (Zero Lock-in)** | ✅ (`cargo rullst eject`) | ❌ | ❌ | ❌ | ❌ |

---

### 💡 The Rullst Philosophy

Unlike other frameworks, Rullst strives to be **simultaneously simple and complete**, with a relentless focus on **security** and **developer experience (DX)**.

The origins of this philosophy can be traced back to the very creation of the Rust programming language. The story goes that Graydon Hoare, the original creator of Rust, lived in an apartment building with an elevator that kept crashing due to software bugs in its underlying C/C++ code. Frustrated by having to climb the stairs because of memory safety vulnerabilities, he set out to create a language that was incredibly fast, yet guaranteed memory safety by design—so that developers could build things that "just worked" without fear.

Rullst was forged with this exact mindset. We believe that web development shouldn't be a constant struggle against the framework, the language, or runtime bugs. Rullst is built for those who want to build with ease and safety, harnessing the raw speed and resource efficiency of Rust.

### Our Core Tenets

1. **Simple yet Complete:** We solve the hardest web development problems out-of-the-box securely (routing, auth, ORM, background jobs, hot-reloading), without sacrificing simplicity or completeness. You shouldn't have to piece together 15 different micro-libraries just to build a secure SaaS.

2. **Built for Humans and AIs:** Rullst is architected to be highly legible and free of runtime "magic". By heavily utilizing static dispatch and compile-time guarantees, the codebase is transparent. This empowers both human developers and AI coding agents to collaborate and build production-ready systems rapidly, even without deep prior framework knowledge.

Rullst is not just a tool; it is a commitment to **Emotional Productivity**. We take care of the boilerplate and the security pitfalls so you can focus entirely on creating value.

<br>

<div align="center">
  <p><i>"All glory and honor to God יהוה in the name of Yeshua the Messiah (Jesus Christ)."</i></p>
</div>
