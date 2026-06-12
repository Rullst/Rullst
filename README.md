<p align="center">
  <img src="https://raw.githubusercontent.com/venelouis/Rullst/main/Rullst.png" alt="Rullst Logo" width="300">
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

<br/>

**Rullst** is an opinionated, developer-first full-stack web framework for Rust, obsessively designed for **Emotional Productivity**. It solves the biggest problem in the Rust web ecosystem: the high barrier of entry. With Rullst, you spend your energy building your business, not fighting borrow checkers and manual routing setups.

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

👉 **[Explore the Official Website & Docs](https://venelouis.github.io/Rullst/)**

💬 **[Join the Community on Discord](https://discord.gg/2ntKFtsSjw)**

> **Found a bug?** [Report an Issue](https://github.com/venelouis/Rullst/issues)
