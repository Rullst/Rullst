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

**Rullst** is an opinionated, developer-first full-stack web framework for Rust, obsessively designed for **Emotional Productivity**. It solves the biggest problem in the Rust web ecosystem: the high barrier of entry. With Rullst, you spend your energy building your business, not fighting borrow checkers and manual routing setups.

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
| **Continuous Fuzzing** | [![Continuous Fuzzing](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/fuzzing.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/fuzzing.yml) | Fuzzing against edge cases & panics |
| **CodeQL SAST** | [![CodeQL SAST](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/codeql.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/codeql.yml) | Advanced semantic code analysis |
| **OWASP ZAP DAST** | [![OWASP ZAP DAST](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/dast-zap.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/dast-zap.yml) | Dynamic vulnerability scanning |
| **Cargo Deny** | [![Cargo Deny](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/cargo-deny.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/cargo-deny.yml) | Banning unmaintained/vulnerable crates |

</div>

<!--
| **Mutation Testing** | [![Mutation Testing](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/mutants.yml?style=flat-square&label=)](https://github.com/Rullst/Rullst/actions/workflows/mutants.yml) | Validating test suite exhaustiveness |
-->

<br>
<h2 align="center"> CLI ⚡ Rullst Framework ⚡ </h2>
<p align="center">
  <img src="gifs/gif.gif" alt="Rullst CLI Initiating LMS Blueprint" width="100%"/>
</p>

<h2 align="center">Click to Watch: How to build a SaaS Blueprint with Rullst </h2>
<p align="center">
<a href="https://www.youtube.com/watch?v=nDXLeNM327g">
  <img src="https://img.youtube.com/vi/nDXLeNM327g/hqdefault.jpg" alt="How to build a SaaS with Rullst" width="430" />
</a>
</p>

<p align="center">Rullst LMS Blueprint from CLI
  <img src="gifs/gif2.gif" alt="Navigating Rullst LMS Blueprint" width="100%" />
</p>


---

### ⚡ Unmatched Performance

Rullst's "Zero-Cost Abstraction" architecture provides full-stack productivity without sacrificing bare-metal speed. In our official [Criterion micro-benchmarks](BENCHMARKS.md):

- **SSR Rendering**: `~1.07 µs` (4.2x faster than Dioxus, 8.5x faster than Leptos).
- **Routing**: `~974 ns` (Identical latency to raw Axum).

### ✨ The "Wow" Factor

Rullst brings the ergonomics of Laravel and Ruby on Rails to the blazing-fast, memory-safe world of Rust:

- **Rullst Nexus**: An auto-generated, dark-mode CMS & Admin Panel directly from your Structs.
- **Hot-Reloading**: Sub-second native DLL hot-swapping. Change your Rust code and see it instantly.
- **Zero-Panic Policy**: Hardened architecture built for production edge infrastructure.
- **Interactive Scaffolding**: 1-click generators for Auth, ERPs, Uptime Monitors, and Deployments.

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

### 📚 Documentation & Community

We've rewritten our entire documentation from scratch into a beautiful, high-performance website. Discover everything Rullst can do, read the benchmarks, and master the framework:

👉 **[Explore the Official Website & Docs](https://rullst.github.io/#docs)**

💬 **[Join the Community on Discord](https://discord.gg/2ntKFtsSjw)**

> **Found a bug?** [Report an Issue](https://github.com/Rullst/Rullst/issues)
