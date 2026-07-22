<p align="center">
  <img src="https://raw.githubusercontent.com/Rullst/Rullst/main/Rullst.png" alt="Rullst Logo" width="300">
</p>

<h1 align="center">Rullst 📜🦀🌐🚀</h1>
<h3 align="center"><i>Rust for those who want to build, not suffer.</i></h3>

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

👉 **[Explore the Official Website & Docs](https://rullst.github.io/#docs)**

💬 **[Join the Community on Discord](https://discord.gg/2ntKFtsSjw)**

> **Found a bug?** [Report an Issue](https://github.com/Rullst/Rullst/issues)

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
| **Matrix DB Tests** | <a href="https://github.com/Rullst/rullst-orm/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/rullst-orm/ci.yml?style=flat-square&label=" alt="Testcontainers" /></a> | Dockerized PostgreSQL & MySQL integration tests |
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
| **SLSA Level 3** | [![SLSA 3](https://img.shields.io/badge/SLSA-Level_3-brightgreen?style=flat-square&label=)](https://slsa.dev/) | Supply-chain Levels for Software Artifacts |
| **Panic Policy** | [![Zero Panics Policy](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/zero-panics.yml?style=flat-square&label=Zero%20Panics)](https://github.com/Rullst/Rullst/actions/workflows/zero-panics.yml) | Graceful error handling across the framework |
| **MSRV** | [![MSRV](https://img.shields.io/badge/MSRV-1.94.0-orange?style=flat-square&label=)](https://github.com/Rullst/Rullst) | Minimum Supported Rust Version |

</div>


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

Rullst's "Zero-Cost Abstraction" architecture provides full-stack productivity without sacrificing bare-metal speed. In our official [Criterion micro-benchmarks](BENCHMARKS.md):

- **SSR Rendering**: `~1.07 µs` (4.2x faster than Dioxus, 8.5x faster than Leptos).
- **Routing**: `~974 ns` (Identical latency to raw Axum).

### ✨ The "Wow" Factor

Rullst brings the ergonomics of Laravel and Ruby on Rails to the blazing-fast, memory-safe world of Rust:

- 🚀 **Hybrid Hot-Reloading**: Sub-millisecond UI updates via WebSockets, paired with Zero-downtime Dynamic Library (`.dll`/`.so`) hot-swapping for backend business logic.
- 🎨 **Rullst Nexus**: An auto-generated, dark-mode CMS & Admin Panel directly from your Structs.
- 🛡️ **Zero-Panic Policy**: Hardened architecture built for production edge infrastructure.
- ⚡ **Interactive Scaffolding**: 1-click generators for Auth, ERPs, Uptime Monitors, and Deployments.

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
* **Where Rullst Excels:** **Batteries Included.** Rullst actually uses *Axum* under the hood for its HTTP routing! But instead of leaving you in an empty room, Rullst gives you a fully furnished house. You get a CLI, ORM, Auth, Stripe integration, and Background Workers out-of-the-box in 1 minute.

### 🚂 Full-Stack Frameworks (Loco, Topcoat)
**Loco** is a fantastic full-stack framework heavily inspired by Rails. It also uses Axum and provides great generators.
**Topcoat** is an experimental, batteries-included framework from the Tokio team that focuses on reactive server-side rendering (SSR) without writing JavaScript.
* **Where Rullst Excels:** **Emotional Productivity & DX.** Rullst takes a radically opinionated stance on Developer Experience. We provide immersive CLI interactive dashboards (`cargo rullst studio`), built-in Wasm Islands, zero-panic architectural guarantees, Nix reproducibility, and native Omni (Desktop/Mobile via Tauri) scaffolding. If you want the absolute easiest, most visually pleasing DX in Rust, Rullst is your home.

### 🎨 Isomorphic Full-Stack Frameworks (Dioxus, Leptos)
These are cutting-edge frameworks that let you write both frontend and backend in a single Rust file using Server Functions and SSR (similar to Next.js or Nuxt).
* **The Catch:** They are heavily **Frontend/Component-Driven**. Your server's primary job is to hydrate and serve UI components. If you need a traditional backend architecture (dedicated Workers, Stripe webhooks, robust ORM migrations, pure REST APIs for mobile apps), an isomorphic model can sometimes feel restrictive or overly coupled to the UI.
* **Where Rullst Excels:** **Architectural Freedom & Synergy.** Rullst is an **API-First / Traditional Full-Stack** (like Rails or Laravel). It gives you an uncompromised, heavy-duty backend layer. But we don't compete with Dioxus/Leptos/Tauri—we *embrace* them! Rullst allows you to use Dioxus for your frontend natively via Wasm Islands (`cargo rullst build:client`), or package your entire application into Desktop & Mobile apps via **Tauri** (`cargo rullst make:omni`).

### 📊 The Full-Stack Feature Matrix

| Feature | **Rullst** | **Loco** | **Topcoat** | **Dioxus / Leptos** | **Axum / Actix** |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **HTTP & Routing** | ✅ | ✅ | ✅ | ✅ (SSR) | ✅ |
| **Built-in ORM** | ✅ (Rullst-ORM) | ✅ (SeaORM) | ✅ (Toasty) | ❌ | ❌ |
| **Interactive CLI Dashboard** | ✅ (Rullst Studio) | ❌ | ❌ | ❌ | ❌ |
| **Auto-Generated Admin Panel**| ✅ (Rullst Nexus) | ❌ | ❌ | ❌ | ❌ |
| **Wasm Islands (Frontend)** | ✅ (Pure Rust) | ❌ | ❌ | ✅ (Core focus) | ❌ |
| **Reactive SSR (No-JS)** | ✅ (Pure Rust) | ❌ | ✅ (Signals)| ❌ | ❌ |
| **Mobile/Desktop Apps** | ✅ (Tauri Integration)| ❌ | ❌ | ✅ (Dioxus) | ❌ |
| **Hot-Reloading** | ✅ (Built-in) | ❌ | ❌ | ✅ (Dioxus) | ❌ |
| **Zero-Panics Policy** | ✅ (Enforced) | ❌ | ❌ | ❌ | ❌ |
| **TypeScript SDK Generator** | ✅ (Built-in) | ❌ | ❌ | ❌ | ❌ |
| **OpenTelemetry Integration** | ✅ (Built-in) | ❌ | ❌ | ❌ | ❌ |

---

### 💡 The Rullst Philosophy

Unlike other frameworks, Rullst strives to be **simultaneously simple and complete**, with a relentless focus on **security** and **developer experience (DX)**.

The origins of this philosophy can be traced back to the very creation of the Rust programming language. The story goes that Graydon Hoare, the original creator of Rust, lived in an apartment building with an elevator that kept crashing due to software bugs in its underlying C/C++ code. Frustrated by having to climb the stairs because of memory safety vulnerabilities, he set out to create a language that was incredibly fast, yet guaranteed memory safety by design—so that developers could build things that "just worked" without fear.

Rullst was forged with this exact mindset. We believe that web development shouldn't be a constant struggle against the framework, the language, or runtime bugs. Rullst is built for those who want to build with ease and safety, harnessing the raw speed and resource efficiency of Rust.

### Our Core Tenets

1. **Simple yet Complete:** We solve the hardest web development problems out-of-the-box securely (routing, auth, ORM, background jobs, hot-reloading), without sacrificing simplicity or completeness. You shouldn't have to piece together 15 different micro-libraries just to build a secure SaaS.

2. **Built for Humans and AIs:** Rullst is architected to be highly legible and free of runtime "magic". By heavily utilizing static dispatch and compile-time guarantees, the codebase is transparent. This empowers both human developers and AI coding agents to collaborate and build production-ready systems rapidly, even without deep prior framework knowledge.

Rullst is not just a tool; it is a commitment to **Emotional Productivity**. We take care of the boilerplate and the security pitfalls so you can focus entirely on creating value.
