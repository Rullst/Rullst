# Welcome to Rullst Framework!

Rullst is a modular full-stack web framework for Rust built on Tokio, Axum, and
SQLx. Version 12 is under active development; use the capability and release
documents to distinguish implemented behavior from roadmap work.

> "Pragmatic, built for developer happiness, and production-minded."

## Documentation Hub

- [✨ Why Rullst?](./why-Rullst.md)
- [📖 Getting Started & Blueprints Showcase](./1-getting-started.md)
- [🤖 Rullst AI: Developing with Autonomous Agents](./2-rullst-ai.md)
- [📊 Rullst Studio: Real-Time Monitoring & Radar](./3-rullst-studio.md)
- [⚙️ Rullst Nexus: Explicit Admin CMS](./4-rullst-nexus.md)
- [💳 Rullst Capital: SaaS Billing & Revenue Dashboard](./5-rullst-capital.md)
- [🧠 Integrating AI into Rullst](./6-ai-integration-tutorial.md)
- [🍳 Rullst Cookbook & Tutorials](./tutorials/01-hello-world.md)
- [📜 Framework Spec](./spec.md)
- [🗺️ Blueprints Roadmap](./blueprints_roadmap.md)
- [💻 CLI Reference](./cli_reference.md)
- [🛡️ Audit Report](https://github.com/Rullst/Rullst/blob/main/AUDIT.md)
- [📦 View on Crates.io](https://crates.io/crates/rullst)

### Why Rullst?
- 🚀 **Measured performance:** Criterion suites and CI track regressions;
  application latency must be measured on the target workload.
- 🧩 **Server-rendered UI:** `html!` and HTMX-oriented scaffolds are the audited
  default. LiveView and Wasm Island primitives exist, while turnkey
  Leptos/Dioxus adapters remain roadmap work.
- 📖 **Interactive Scalar Docs (`/docs`):** Built-in OpenAPI UI via `cargo rullst make:scalar`.
- ☸️ **Kubernetes Native:** Cloud-Native manifest scaffolding (`cargo rullst make:k8s`) with `/health` & `/ready` probes.
- 📡 **Rullst Radar & Prometheus:** Process/Tokio observations and a
  Prometheus text exporter, with unavailable probes reported explicitly.
- 🛡️ **Local AI option:** Use an Ollama endpoint when prompts should stay
  on infrastructure you control; network isolation remains a deployment duty.
- 📦 **Batteries Included:** ORM (rullst-orm), Auth, Revenue Dashboard, Jobs, Mail, Scheduler, Cache, Security.
