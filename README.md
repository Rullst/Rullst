<div align="center">
  <p><i>All glory and honor to God יהוה in the name of Yeshua the Messiah (Jesus Christ).</i></p>
</div>

<p align="center">
  <img src="https://raw.githubusercontent.com/Rullst/Rullst/main/Rullst.png" alt="Rullst Logo" width="300">
</p>

<h1 align="center">🌐🦀📜 Rullst 📜🦀🌐</h1>
<h3 align="center"><i>Intelligent, Effortless and Highly Secure Rust Framework - Because With Rullst We Rule!</i></h3>

<p align="center">
  <a href="https://crates.io/crates/rullst"><img src="https://img.shields.io/crates/v/rullst?style=for-the-badge&color=10b981&logo=rust" alt="Crates.io"></a>
  <a href="https://crates.io/crates/rullst"><img src="https://img.shields.io/crates/d/rullst?style=for-the-badge&color=blue" alt="Crates.io Downloads"></a>
  <a href="https://docs.rs/rullst"><img src="https://img.shields.io/docsrs/rullst?style=for-the-badge&logo=docsdotrs" alt="Docs.rs"></a>
  <a href="https://github.com/Rullst/Rullst/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/ci.yml?style=for-the-badge&label=Build" alt="Rust CI"></a>
  <img src="https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge" alt="License: MIT">
</p>

<h3 align="center">🛡️ Security Engineering & CI Visibility</h3>

<p align="center">
  Rullst applies a zero-panic policy to production paths and uses layered CI checks.<br/>
  Badges report individual workflow results; they are not a security certification or a guarantee for a deployment.
</p>

<div align="center">

| Security Audit | Status | Description |
| :--- | :---: | :--- |
| **OSSF Scorecard** | [![OSSF Scorecard](https://img.shields.io/ossf-scorecard/github.com/Rullst/Rullst?label=&style=flat-square)](https://github.com/Rullst/Rullst/actions/workflows/scorecards.yml) | Supply-chain security & best practices |
| **Codecov** | [![Codecov](https://img.shields.io/codecov/c/github/Rullst/Rullst?style=flat-square&label=)](https://codecov.io/gh/Rullst/Rullst) | Coverage reporting for exercised code |
| **OpenSSF** | [![OpenSSF](https://img.shields.io/badge/status-passing-brightgreen?style=flat-square&label=)](https://www.bestpractices.dev/projects/13321) | Open source security standards |
| **Matrix DB Tests** | [![Matrix DB Tests](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/ci.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/ci.yml) | Dockerized PostgreSQL & MySQL integration tests |
| **Continuous Fuzzing** | [![Continuous Fuzzing](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/fuzzing.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/fuzzing.yml) | Fuzzing against edge cases & panics |
| **Property Testing** | [![Property Testing](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/proptest.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/proptest.yml) | Validating complex logic against edge cases |
| **CodeQL SAST** | [![CodeQL SAST](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/codeql.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/codeql.yml) | Advanced semantic code analysis |
| **OWASP ZAP DAST** | [![OWASP ZAP DAST](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/dast-zap.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/dast-zap.yml) | Dynamic vulnerability scanning |
| **Cargo Deny** | [![Cargo Deny](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/cargo-deny.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/cargo-deny.yml) | Dependency policy with documented, expiring exceptions |
| **Cargo Audit** | [![Cargo Audit](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/audit.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/audit.yml) | Continuous scanning for crate vulnerabilities |
| **Cargo Geiger** | [![Cargo Geiger](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/unsafe-policy.yml?style=flat-square&label=Geiger)](https://github.com/Rullst/Rullst/actions/workflows/unsafe-policy.yml) | Unsafe code counting & memory safety auditing |
| **Cargo SemVer** | [![SemVer Checks](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/semver.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/semver.yml) | Strict SemVer API breakage checks |
| **Cargo Machete** | [![Cargo Machete](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/machete.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/machete.yml) | Detecting unused and bloated dependencies |
| **Benchmark CI** | [![Benchmark CI](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/bench.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/bench.yml) | Continuous performance regression testing |
| **Snapshot Testing** | [![Snapshot Testing](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/ci.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/ci.yml) | UI & Macro structural regression testing |
| **E2E Smoke Verification** | [![E2E Smoke](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/e2e-smoke.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/e2e-smoke.yml) | Live HTTP server boot and real client request assertions |
| **Concurrency Sanitizers** | [![Sanitizers](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/sanitizers.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/sanitizers.yml) | Data-race (TSan) and memory corruption (ASan) detection |
| **Spellcheck CI** | [![Spellcheck CI](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/spellcheck.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/spellcheck.yml) | Automated typo detection across docs and code |
| **Clippy Lints** | [![Clippy Lints](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/ci.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/ci.yml) | Strict compiler & style linting |
| **Unsafe Policy** | [![Unsafe Policy](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/unsafe-policy.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/unsafe-policy.yml) | Audits a narrow, documented allowlist of OS/FFI unsafe blocks |
| **Miri UB Detection** | [![Miri](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/miri.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/miri.yml) | Detecting Undefined Behavior and memory leaks |
| **Kani Verifier** | [![Kani Verifier](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/kani.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/kani.yml) | Model checking for explicitly defined harnesses |
| **Mutation Testing** | [![Mutation Testing](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/mutants.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/mutants.yml) | Mutation testing for test suite robustness |
| **Architecture Linter** | [![TangleGuard](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/tangleguard.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/tangleguard.yml) | Enforcing architectural boundaries |
| **Release Provenance** | [![Release Provenance](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/release.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/release.yml) | Build provenance attestations for release artifacts; no SLSA level is claimed here |
| **Panic Policy** | [![Zero Panics Policy](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/zero-panics.yml?style=flat-square&label=Zero%20Panics)](https://github.com/Rullst/Rullst/actions/workflows/zero-panics.yml) | Graceful error handling across the framework |
| **Secret Scanning** | [![Trufflehog](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/trufflehog.yml?style=flat-square&label=Trufflehog)](https://github.com/Rullst/Rullst/actions/workflows/trufflehog.yml) | Automated CI prevention of leaked credentials |
| **no_std Build Check** | [![no_std Build](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/no_std-build.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/no_std-build.yml) | Validates `rullst-iot` compiles on STM32, ESP32-C3, Cortex-M bare-metal targets |
| **Wasm & Edge Matrix** | [![Wasm Matrix](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/wasm-matrix.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/wasm-matrix.yml) | Continuous compilation matrix for `wasm32-unknown-unknown` and `wasm32-wasip1` |
| **AI PR Sentinel Audit** | [![AI Sentinel](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/ai-sentinel-pr.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/ai-sentinel-pr.yml) | Automated AST IDOR scanning, RBAC checks, and SBOM on Pull Requests |
| **Fuzz Corpus Minimization** | [![Corpus Sync](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/corpus-sync.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/corpus-sync.yml) | Weekly seed-corpus compaction for the configured fuzz targets |
| **Cargo Udeps** | [![Cargo Udeps](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/udeps.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/udeps.yml) | Compiler AST unused dependencies & dead feature flags detector |
| **CycloneDX SBOM** | [![SBOM](https://img.shields.io/badge/SBOM-CycloneDX_1.5-blue?style=flat-square&label=)](https://cyclonedx.org/) | Automated Software Bill of Materials with SHA-256 package hashes |
| **Network Surface Audit** | [![Network Scan](https://img.shields.io/badge/Network_Audit-RustScan_Mode-blueviolet?style=flat-square&label=)](https://github.com/Rullst/Rullst) | High-speed local port & interface binding scanner |
| **Git Pre-Commit Hook** | [![Pre-Commit](https://img.shields.io/badge/Pre--Commit-Active-success?style=flat-square&label=)](https://github.com/Rullst/Rullst) | Local gatekeeper enforcing rustfmt, clippy, and IDOR audits |
| **IoT Integration** | [![IoT Integration](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/iot-integration.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/iot-integration.yml) | Telemetry helpers, signed OTA manifests, and `no_std` builds |
| **Security Audit** | [![Security Audit](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/security-audit.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/security-audit.yml) | RustSec advisory database CVE scanning |
| **Toolchain Doctor** | [![Doctor](https://img.shields.io/badge/Doctor-System_Diagnostics-teal?style=flat-square&label=)](https://github.com/Rullst/Rullst) | Unified health-check for MSRV, Docker, Kani, and security linters |
| **Rustls Paths** | [![Rustls](https://img.shields.io/badge/TLS-Rustls_Paths-10b981?style=flat-square&label=)](https://github.com/rustls/rustls) | Rustls-backed transport on supported paths; verify the complete application dependency tree and TLS termination |
| **IoT Crypto Containment** | [![IoT Crypto Containment](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/pqc-compliance.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/pqc-compliance.yml) | Signed OTA tests and explicit simulator-boundary checks; no PQC/HSM compliance claim |
| **MSRV** | [![MSRV](https://img.shields.io/badge/MSRV-1.96.0-orange?style=flat-square&label=)](https://github.com/Rullst/Rullst) | Minimum Supported Rust Version |

</div>

> 📖 **[Read the detailed breakdown of all our CI/CD Security Workflows here](https://github.com/Rullst/Rullst/blob/main/WORKFLOWS.md)**
>
> 🧭 **[Capability Status & Vision Decisions](https://github.com/Rullst/Rullst/blob/main/docs/src/capability-ledger.md)** preserves ambitious features that are partial or not implemented, with an explicit recommendation and rationale for each one.
<br>

## 💡 The Rullst Philosophy

Unlike other frameworks, Rullst strives to be **simultaneously simple and complete**, with a relentless focus on **intelligence**, **security** and **developer experience (DX)**.

The origins of this philosophy can be traced back to the very creation of the Rust programming language. The story goes that Graydon Hoare, the original creator of Rust, lived in an apartment building with an elevator that kept crashing due to software bugs in its underlying C/C++ code. Frustrated by having to climb the stairs because of memory safety vulnerabilities, he set out to create a language that was incredibly fast, yet guaranteed memory safety by design—so that developers could build things that "just worked" without fear.

Rullst was forged with this exact mindset: simplicity, intelligence, security and effortless development. We believe that development shouldn't be a constant struggle against the framework, the language, or runtime bugs. Rullst is built for those who want to build with ease and safety, harnessing the raw speed and resource efficiency of Rust.

### Our Core Tenets

1. **Coordinated, not magical:** Rullst integrates routing, auth, ORM, bounded background jobs, and developer tooling behind explicit APIs. Generated defaults reduce setup, but every deployed application's security and operations still require review.

2. **Built for Humans and AIs:** Rullst is architected to be legible and explicit, with static dispatch and compile-time generation where practical. This helps human developers and coding agents collaborate on systems whose production boundaries can be reviewed and tested.

Rullst is not just a tool; it is a commitment to **Intelligence, Security and Emotional Productivity**. We take care of the boilerplate and the security pitfalls so you can focus on what matters the most: creating value.

---

### ⚡ Quick Start: From Zero to Hero in 2 Minutes

Never programmed in Rust before? No problem! Follow these simple steps to go from zero to a running web application:

#### 1️⃣ Step 1: Install Rust & Cargo
Rullst runs on Rust. If you don't have Rust installed yet, install it using the official toolchain installer:

- **Linux & macOS**:
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **Windows**:
  Download and run the installer from **[rustup.rs](https://rustup.rs)** or run in PowerShell:
  ```powershell
  winget install --id Rustlang.Rustup
  ```

> *Tip: Restart your terminal and verify the installation by running `cargo --version`.*

---

#### 2️⃣ Step 2: Create Your Web Project
In your terminal, choose how you want to create your project:

```bash
# Option A: Create a new folder named 'my_app' and enter it
cargo new my_app
cd my_app

# Option B: If you already created and opened an empty folder in your terminal/VS Code:
cargo init
```

> ⚠️ **Important**: Always make sure your terminal is inside the project folder containing the generated `Cargo.toml` file!
> 
> ```text
> my_app/
> ├── Cargo.toml    # (Dependencies and project metadata)
> └── src/
>     └── main.rs   # (Your server application entry point)
> ```

---

#### 3️⃣ Step 3: Add Rullst to Your Project
Run these commands in your terminal (inside your project folder) to add Rullst and the Tokio runtime:

```bash
cargo add rullst
cargo add tokio --features full
```
---

#### 4️⃣ Step 4: Add the Code (`src/main.rs`)
Open `src/main.rs` in your code editor and replace its entire contents with the Hello World code below:

### 💻 The Beauty of Rullst (Hello World)

Build modern, type-safe full-stack web applications with zero client JS bundles, built-in OWASP security, and sub-millisecond cold starts:

```rust
use rullst::{html, response::Html, routes, Server};

// 1. Type-Safe Server-Side Rendered View with JSX-like compile-time syntax
async fn home() -> Html<String> {
    Html(html! {
        <div class="h-screen bg-slate-900 text-emerald-400 flex items-center justify-center">
            <h1 class="text-4xl font-bold">"Hello, Rullst!"</h1>
        </div>
    })
}

#[tokio::main]
async fn main() {
    // 2. Declarative, zero-reflection route dispatching
    let app = routes![
        get("/" => home)
    ];

    // 3. Launch high-throughput Tokio async HTTP server on port 3000
    Server::new(app)
        .run(3000)
        .await
        .unwrap();
}
```

#### 🔍 What makes this powerful:
- **⚡ Zero-Bundle SSR (`html!`)**: Compile-time JSX-like syntax that generates blazing fast static strings with automatic XSS sanitization and zero virtual-DOM overhead.
- **🛣️ Expressive Routing (`routes!`)**: Clean declarative macro mapping directly to Tokio/Axum static dispatch without runtime reflection.
- **🛡️ Secure Kernel Defaults (`Server`)**: Installs a strict nonce-based header baseline, double-submit CSRF protection, and live telemetry probes; deployment-specific TLS, proxy, origin, and scanner validation remain the application's responsibility.

---

#### 5️⃣ Step 5: Run Your Application! 🚀
Inside your project folder (where `Cargo.toml` is located), start the server:

```bash
cargo run
```

Open **`http://localhost:3000`** in your browser to see your high-performance web application running live! 🎉

> 💡 **Pro-Tip (CLI Scaffolding)**: Want a complete MVC boilerplate with database migrations, JWT auth, and Docker setup? Use the official CLI generator:
> ```bash
> cargo install cargo-rullst
> cargo rullst
> ```

<h2 align="center"> CLI ⚡ Rullst Framework ⚡ </h2>
<p align="center">
  <img src="https://raw.githubusercontent.com/Rullst/Rullst/main/images/gifs/gif.gif" alt="Rullst CLI Initiating LMS Blueprint" width="80%"/>
</p>

<h2 align="center">Click to Watch: How to build a SaaS Blueprint with Rullst </h2>
<p align="center">
<a href="https://www.youtube.com/watch?v=nDXLeNM327g">
  <img src="https://img.youtube.com/vi/nDXLeNM327g/hqdefault.jpg" alt="How to build a SaaS with Rullst" width="60%" />
</a>
</p>

<table align="center" width="100%">
  <tr>
    <th align="center" width="50%"><h2>SaaS Blueprint</h2></th>
    <th align="center" width="50%"><h2>LMS Blueprint</h2></th>
  </tr>
  <tr>
    <td align="center">
      <img src="https://raw.githubusercontent.com/Rullst/Rullst/main/images/gifs/gif1.gif" alt="SaaS Blueprint" width="100%" />
    </td>
    <td align="center">
      <img src="https://raw.githubusercontent.com/Rullst/Rullst/main/images/gifs/gif2.gif" alt="LMS Blueprint" width="100%" />
    </td>
  </tr>
</table>

---
<br>

<p align="center">
  <img src="https://raw.githubusercontent.com/Rullst/Rullst/main/images/cargo-rullst-dash.png" alt="Rullst Interactive Terminal Dashboard" width="100%"/>
</p>

<p align="center">
  <img src="https://raw.githubusercontent.com/Rullst/Rullst/main/images/Rullst-Omni.png" alt="Rullst Omni Mobile & Desktop Simulator" width="100%"/>
</p>

### 📚 Documentation & Community

We've rewritten our entire documentation from scratch into a beautiful, high-performance website. Discover everything Rullst can do, read the benchmarks, and master the framework:

👉 **[Explore the Official Website & Docs](https://rullst.github.io)**

💬 **[Join the Community on Discord](https://discord.gg/2ntKFtsSjw)**

> **Found a bug?** [Report an Issue](https://github.com/Rullst/Rullst/issues)

---

### ⚡ Unmatched Performance

Rullst's "Zero-Cost Abstraction" architecture provides full-stack productivity without sacrificing bare-metal speed:

- **SSR HTML5 Rendering**: Zero-bundle static string rendering avoiding Virtual DOM allocations.
- **Macro Routing (`routes!`)**: Direct compile-time static dispatch powered by Axum and Tokio.
- **HtmlSanitizer & XSS Shield**: High-speed Ammonia AST payload filtering.
- **RbacGuard Role & Ownership BOLA**: Explicit role and ownership authorization helpers.
- **Vault In-Memory Zeroization**: Cryptographic drop memory wiping preventing cold-boot RAM inspection.
- **Stripe Webhook Signature Verification**: Constant-time HMAC-SHA256 protecting against timing attacks.
- **Passkey WebAuthn Challenge Parser**: High-performance FIDO2 passwordless auth parsing.
- **Device Fingerprinting Helper**: Subnet-aware session-binding signal; it has runtime cost and must not be the sole authentication factor.
- **AI Guardrail Prompt Sanitizer**: In-memory prompt injection neutralization before LLM transport.
- **RAG Cosine Vector Similarity**: SIMD-accelerated local vector embedding similarity computation.

> 📊 **Explore live results & reproducible suites:**
> - [**📈 Interactive Benches Dashboard**](https://rullst.github.io/Rullst/#benches) — Real-time telemetry and microsecond visualizers on the official site.
> - [**⚖️ Comparative Benchmarks**](https://rullst.github.io/Benchmarks/) — Open benchmark repository with reproducible TechEmpower-style setups, Criterion HTML reports, memory profiling, and continuous performance regression CI.

- 🚀 **Hybrid Hot-Reloading & Fast Linkers**: Optional `mold`/`lld` configuration and WebSocket-driven development reloads; build latency depends on the host and project.
- 🎨 **Developer Control Room & Nexus CMS**: An all-in-one Web Suite (`cargo rullst studio` at `:5555`) with Data Browser, Visual Threat Radar, Real-time Metrics, and auto-generated Admin Panels (`/nexus`) from your Structs.
- 🛡️ **RASP Engine & Pre-Controller Shield**: Bounded, heuristic request inspection for common XSS, SQLi, traversal, and command-injection indicators before handlers; authorization remains an explicit application policy.
- 🔑 **Passkeys & WebAuthn (FIDO2)**: Ceremony and signature-verification helpers for compatible authenticators; deployments must configure RP/origin policy and persist challenges/counters atomically.
- 🌐 **Provider-Agnostic AI (Local & Cloud)**: Connect to Ollama-compatible/local endpoints or supported cloud providers (Gemini, Claude, OpenAI, DeepSeek) through a guarded high-level client.
- 💳 **Capital SaaS Ledger & Billing**: Real-time MRR/ARR analytics and constant-time HMAC signature verification for Stripe and LemonSqueezy webhooks.
- 🔐 **Field Encryption & Memory Hygiene**: Versioned AES-GCM field encryption (`#[orm(encrypted)]`) and `Zeroize` for selected secret buffers; key management and OS-level memory exposure remain deployment concerns.
- 🔄 **Expressive Active Record Transactions**: Borrow-checker safe `User::transaction(|tx| async move { ... })` with automatic task-local scoping (`CURRENT_TX`), commit-on-success, and rollback-on-error behavior.
- 🔄 **Reverse ORM Scaffolding**: Automatically reverse-engineer Rust `struct` models from existing database tables using `cargo rullst make:models-from-db`.
- 🔍 **Static CLI Inspection**: Inspect active route tables, ORM models, and JSON schemas directly in the terminal via `cargo rullst inspect`.
- 🛡️ **Zero-Panic Policy**: Production library paths are gated against `panic!`, `unwrap`, and `expect`; this is an engineering policy, not a guarantee that software or dependencies can never terminate unexpectedly.
- ⚡ **Interactive Scaffolding**: Generators for Auth, ERP/SaaS starting points, monitors, and deployment manifests (`cargo rullst deploy`); generated output requires application review.

---

### 🌐 Frontend modes

Rullst scaffolds several presentation strategies. They are alternatives, not a
claim that every mode has identical maturity, bundle size, or migration cost.

| Mode | Runtime model | Important boundary |
| :--- | :--- | :--- |
| HTMX SSR | Server-rendered HTML with HTMX interactions | HTMX is a browser dependency even though no project-local SPA bundle is required. |
| LiveView | Server state synchronized over WebSockets | Requires connection lifecycle, origin checks, backpressure, and reconnect testing. |
| Wasm Islands | Client-side WebAssembly for selected components | Bundle size and browser compatibility must be measured per application. |
| Pico.css | Semantic server-rendered HTML with an external stylesheet | Styling is external; application behavior remains server-oriented. |
| Tera | File-based server templates | Template context and escaping boundaries require review like any rendering system. |

---

### 🔓 Standard Axum & SQLx Escape Hatches

Rullst is built on **Axum**, **Tokio**, **Tower**, and **SQLx**, and exposes standard routers and pools at important integration points. Some framework helpers and generated structures still require an explicit migration when ejecting:

- **Incremental Adoption:** Mount existing `axum::Router` instances directly into `rullst::server::Server`.
- **Standard SQLx:** Run raw `sqlx::Pool` queries alongside `rullst-orm` without wrappers.
- **Escape Hatch:** Use the CLI eject output as a migration starting point and review the generated code before deployment.
- 📖 Read the full [Axum & SQLx Migration & Escape Hatch Guide](https://github.com/Rullst/Rullst/blob/main/docs/src/axum-sqlx-migration.md).

---

## Choosing Rullst

Rullst is a good fit when an Axum-based application benefits from a coordinated
CLI, ORM, authentication helpers, bounded background workers, provider adapters,
and local developer tooling in one versioned workspace. A smaller HTTP library
may be preferable when the application needs only routing and middleware; a
frontend-first framework may be preferable when client component composition is
the primary architecture.

Evaluate only the features you intend to enable. In particular:

- security middleware is defense-in-depth and does not replace authorization,
  validation, proxy configuration, or penetration testing;
- generated OpenAPI, TypeScript, deployment, and ejection output requires review
  and compilation in the target application;
- Studio is a local developer tool and Nexus requires an explicit authenticated
  access policy;
- live NFS-e, Alipay RSA2, MQTT/HSM/PQC, S3/R2, and Connect message brokers are
  not stable capabilities in version 12.

---

### 🏛️ The Rullst Monorepo (v12.0.0+)

Rullst is a unified monorepo. Core, ORM, Connect, and the domain crates are versioned and tested together so compatibility regressions can be caught in one workspace; consumers should still follow SemVer notes and the release matrix for the exact version they use.

**Explore the Monorepo Ecosystem:**
- 🦀 **[rullst-core](https://github.com/Rullst/Rullst/tree/main/rullst-core)**: Runtime-only-by-default HTTP server, routing engine, and telemetry kernel; ORM and SQLite queues are explicit features.
- 💾 **[rullst-orm](https://github.com/Rullst/Rullst/tree/main/rullst-orm)**: Active Record ORM, automated migrations, and multi-tenancy.
- 🛡️ **[rullst-auth](https://github.com/Rullst/Rullst/tree/main/rullst-auth)**: Passkeys/WebAuthn, Argon2id, encrypted cookie sessions, and RBAC authorization.
- 🔒 **[rullst-security](https://github.com/Rullst/Rullst/tree/main/rullst-security)**: RASP deep inspection, Honeypot bot traps, XSS/CSP sanitization, and HMAC audit log.
- 🤖 **[rullst-ai](https://github.com/Rullst/Rullst/tree/main/rullst-ai)**: Provider-agnostic AI agent engine (Gemini, OpenAI, Claude, DeepSeek, Ollama).
- 💰 **[rullst-capital](https://github.com/Rullst/Rullst/tree/main/rullst-capital)**: SaaS MRR/ARR analytics and payment-provider adapters; live Alipay RSA2 and NFS-e authorization remain fail-closed roadmap work.
- 🔌 **[rullst-connect](https://github.com/Rullst/Rullst/tree/main/rullst-connect)**: OAuth2/OIDC social login with strict discovery, offline fixtures, and rotating JWKS caches. Queue transports currently live in Core.
- 📡 **[rullst-iot](https://github.com/Rullst/Rullst/tree/main/rullst-iot)**: `no_std` telemetry/frame helpers and Ed25519-signed OTA manifest verification; MQTT transport, HSM, and PQC remain roadmap work.
- ✉️ **[rullst-mail](https://github.com/Rullst/Rullst/tree/main/rullst-mail)**: Transactional email delivery engine with anti-phishing and DLP secret scanning.
- 📊 **[rullst-studio](https://github.com/Rullst/Rullst/tree/main/rullst-studio)**: Developer Control Room (`:5555`) with live telemetry and data browser.
- ⚙️ **[rullst-nexus](https://github.com/Rullst/Rullst/tree/main/rullst-nexus)**: Auto-generated Admin CMS (`/nexus`) and SOC Threat Radar.
- 🛠️ **[cargo-rullst](https://github.com/Rullst/Rullst/tree/main/cargo-rullst)**: CLI scaffolding, bounded AST IDOR checks, and deployment helpers.

---

**Rullst** is an opinionated, developer-first AI-powered full-stack framework for Rust, obsessively designed for **Emotional Productivity and Security**. It solves the biggest problem in the Rust web ecosystem: the high barrier of entry. With Rullst, you spend your energy building your business, not fighting borrow checkers and manual routing setups.

---

<br>

<div align="center">
  <p><i>All glory and honor to God יהוה in the name of Yeshua the Messiah (Jesus Christ).</i></p>
</div>
