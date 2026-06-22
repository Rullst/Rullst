<p align="center">
  <img src="https://raw.githubusercontent.com/Rullst/Rullst/main/Rullst.png" alt="Rullst Logo" width="300">
</p>

<h1 align="center">Rullst 📜🦀🌐🚀</h1>
<h3 align="center"><i>Rust for those who want to build, not suffer.</i></h3>

<p align="center">
  <img src="https://img.shields.io/crates/v/rullst?style=for-the-badge&color=10b981&logo=rust" alt="Crates.io">
  <img src="https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge" alt="License: MIT">
  <a href="https://discord.gg/2ntKFtsSjw">
    <img src="https://img.shields.io/badge/Discord-Join%20Community-5865F2?logo=discord&logoColor=white&style=for-the-badge" alt="Discord">
  </a>
  <a href="https://rullst.github.io">
    <img src="https://img.shields.io/badge/Website-Official%20Site-blue?logo=github&logoColor=white&style=for-the-badge" alt="Official Website">
  </a>
</p>

<p align="center">
  <a href="https://github.com/Rullst/Rullst/actions/workflows/dast-zap.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/dast-zap.yml?style=for-the-badge&label=OWASP%20ZAP" alt="OWASP ZAP DAST"></a>
  <a href="https://github.com/Rullst/Rullst/actions/workflows/scorecards.yml"><img src="https://img.shields.io/ossf-scorecard/github.com/Rullst/Rullst?label=OSSF%20Scorecard&style=for-the-badge" alt="OSSF Scorecard"></a>
  <a href="https://www.bestpractices.dev/projects/13321"><img src="https://img.shields.io/cii/level/13321?style=for-the-badge&label=OpenSSF%20Best%20Practices" alt="OpenSSF Best Practices"></a>
  <a href="https://github.com/Rullst/Rullst/actions/workflows/cargo-deny.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/cargo-deny.yml?style=for-the-badge&label=Cargo%20Deny" alt="Cargo Deny"></a>
  <a href="https://github.com/Rullst/Rullst/actions/workflows/coverage.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/coverage.yml?style=for-the-badge&label=LLVM%20Coverage" alt="LLVM Coverage"></a>
  <a href="https://github.com/Rullst/Rullst/actions/workflows/mutants.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/mutants.yml?style=for-the-badge&label=Mutation%20Testing" alt="Mutation Testing"></a>
  <a href="https://github.com/Rullst/Rullst/actions/workflows/fuzzing.yml"><img src="https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/fuzzing.yml?style=for-the-badge&label=Continuous%20Fuzzing" alt="Continuous Fuzzing"></a>
</p>

<br/>

**Rullst** is an opinionated, developer-first full-stack web framework for Rust, obsessively designed for **Emotional Productivity**. It solves the biggest problem in the Rust web ecosystem: the high barrier of entry. With Rullst, you spend your energy building your business, not fighting borrow checkers and manual routing setups.
  
<h2 align="center">How to build a SaaS Blueprint with Rullst </h2>
<p align="center">
<a href="https://www.youtube.com/watch?v=nDXLeNM327g">
  <img src="https://img.youtube.com/vi/nDXLeNM327g/hqdefault.jpg" alt="How to build a SaaS with Rullst" width="430" />
</a>

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

### 🚀 Quick Start

Get your next SaaS up and running in under 60 seconds:

```bash
# 1. Install the CLI
cargo install cargo-rullst

# 2. Create a new full-stack project (Interactive Wizard)
cargo rullst new my-startup

# 3. Start the Hot-Reloading Dev Server
cd my-startup
cargo rullst dev
```

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
