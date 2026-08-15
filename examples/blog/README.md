# The Sovereign SaaS Blog & Publisher 📖⚡

Welcome to **The Sovereign SaaS Blog & Publisher**, the official reference showcase and integration testbed for the **Rullst Framework**.

This application is **100% real and non-mocked**. It demonstrates how all core subsystems of Rullst integrate seamlessly into a single, cohesive, production-grade Rust application.

---

## 🌟 Flagship Subsystems Demonstrated

```mermaid
graph TD
    Nav["Sticky Showcase Navigation Bar"]
    Nav --> F1["1. Zero-Bundle HTMX SSR (/)"]
    Nav --> F2["2. LiveView Server-Driven UI (/live-feed)"]
    Nav --> F3["3. Wasm Island Editor (/editor)"]
    Nav --> F4["4. Pico Semantic CSS (/pico-demo)"]
    Nav --> F5["5. File-Based Templates (/templates-demo)"]
    Nav --> O1["6. Hybrid ORM & Data Mapper (/posts/repository)"]
    Nav --> C1["7. Capital Billing & SPED DPS (/pricing)"]
    Nav --> S1["8. Security RASP Sandbox (/security-demo)"]
    Nav --> A1["9. AI Vector Search (/ai-assistant)"]
    Nav --> CR["10. Studio (/studio) & Nexus (/nexus)"]
```

### 1. ⚡ All 5 Front-End Paradigms Unified in Pure Rust
Rullst is the only full-stack Rust framework that unifies all 5 major web presentation paradigms natively into a single coherent architecture:

- **Zero-Bundle HTMX SSR (`/`)**: Ultra-fast declarative HTML generated at compile time using the `html!` macro and Axum static dispatch with 0 KB JavaScript overhead.
- **LiveView Server-Driven UI (`/live-feed`, `/_live`)**: Real-time state synchronization over persistent Tokio WebSockets (Phoenix LiveView / Dioxus Live pattern) with zero client-side logic.
- **Wasm Reactive Island (`/editor`, `/wasm-counter`)**: Client-side reactive micro-frontend compiled with `wasm-bindgen` (Leptos / Yew WASM & Signals pattern) loaded pontually where heavy client computation is needed.
- **Pico Semantic CSS (`/pico-demo`)**: Zero-Build semantic HTML5 styling powered by Pico.css v2 with automatic OS Dark/Light mode detection, **zero Node.js/NPM builds**, and 0 KB JavaScript.
- **File-Based Classic Templates (`/templates-demo`)**: External Jinja2/Tera `.html` files in `templates/` with full layout inheritance (Django, Rails & Loco.rs pattern).

### 2. 🗄️ Hybrid ORM & Multi-Tenancy
- **Active Record ([`lib.rs`](src/lib.rs))**: Zero-boilerplate data model with task-local SaaS multi-tenancy auto-scoping (`apply_tenant_scope`) responding to the `X-Tenant-ID` header.
- **Repository / Data Mapper ([`repository_demo.rs`](src/repository_demo.rs))**: Decoupled domain aggregations (`PostRepository::get_author_analytics`) executing parameterized SQLx queries.
- **Intent-Based Modeling**: Visualizer of automated index migrations via doc comments (`/// @index(tenant_id, title)`).

### 3. 💳 Capital SaaS Monetization & SPED Fiscal Engine ([`billing_demo.rs`](src/billing_demo.rs))
- **Quota Governance**: Real `Billable::check_quota` evaluation across Community, Pro, and Enterprise tiers.
- **Receita Federal SPED NFS-e Invoicing**: Direct in-memory Declaração de Prestação de Serviços (DPS) XML generation with enveloped **W3C XMLDSig** RSA-SHA256 digital signatures at **R$ 0.00 intermediary fees**.

### 4. 🛡️ Security Sandbox & RASP Inspection ([`security_demo.rs`](src/security_demo.rs))
- **RASP Engine**: Real-time inspection intercepting SQL Injection (`' OR '1'='1`) and Path Traversal (`../../../../etc/passwd`).
- **Anti-Timing Guard**: Constant-time response normalization (250ms target) and synthetic Argon2 CPU cycles preventing user enumeration.
- **AI Firewall v2**: Heuristic prompt shield blocking jailbreaks, DAN overrides, and zero-width unicode poisoning.
- **DLP Secret Masking**: Automatic redacting of sensitive bearer tokens and passwords (`redact_secrets`).
- **Login Jail**: Tarpit engine with progressive async backoff to defeat brute-force login attacks.
- **Honeypot Trap**: Decoy route `/wp-admin` triggering automated alerts to the SOC Threat Radar.

### 5. 🤖 AI RAG & Vector Semantic Search ([`ai_demo.rs`](src/ai_demo.rs))
- **Vector Search**: Real-time **Cosine Similarity** ranking over blog embeddings.
- **Prompt Injection Shield**: Input filter preventing adversarial prompt leakage and jailbreaks.

### 6. 🎛️ Integrated Control Rooms
- **Studio Developer Control Room**: Mounted at [`/studio`](http://127.0.0.1:3000/studio) (Database Inspector, SOC Threat Radar, Capital Revenue, Traces).
- **Nexus Admin CMS**: Mounted at [`/nexus`](http://127.0.0.1:3000/nexus) (Model CRUD, AI Assistant Chat).

---

## 🆚 Frontend Paradigms Matrix: Rullst vs. Ecosystem

| Framework | Primary Engine | Paradigm Architecture | Ideal Use Case |
| :--- | :--- | :--- | :--- |
| **Leptos** | WASM & Signals | Client-side WASM hydration (Single Paradigm) | Rich SPAs with heavy client reactive computation |
| **Dioxus** | Virtual DOM | React-like Rust VDOM (Single Paradigm) | Cross-platform desktop & mobile interfaces |
| **Loco.rs** | Askama / Tera | File-based `.html` templates (Single Paradigm) | Traditional Rails/Django-style MVC monoliths |
| **Topcoat Tokio** | Transpiled Micro-JS | Server-rendered with macro-generated JS snippets | Server-side rendering without WASM |
| **👑 Rullst** | **Sovereign Multi-Engine** | **All 5 Paradigms Natively Supported** (Zero-Bundle HTMX, LiveView WS, Wasm Islands, Pico Semantic CSS, Tera Templates) | **Total freedom**: choose the optimal paradigm per page with zero lock-in |

---

## 🧭 Interactive Route Catalog

| Route | Method | Subsystem | Description |
| :--- | :--- | :--- | :--- |
| `http://localhost:3000/` | `GET` | **HTMX SSR** | Landing feed with Active Record post creation form. |
| `http://localhost:3000/posts` | `POST` | **Active Record** | Saves a new post into SQLite under current tenant. |
| `http://localhost:3000/posts/repository` | `GET` | **Repository ORM** | Data Mapper analytics and Intent-Based `@index` visualizer. |
| `http://localhost:3000/live-feed` | `GET` | **LiveView** | Server-driven UI state synchronization over WebSockets. |
| `http://localhost:3000/editor` | `GET` | **Wasm Island** | Client-side reactive WebAssembly micro-frontend. |
| `http://localhost:3000/pico-demo` | `GET` | **Pico CSS** | Zero-Build semantic HTML5 with automatic Dark/Light mode and 0 KB JS. |
| `http://localhost:3000/templates-demo` | `GET` | **Tera Templates** | External Jinja2/Tera template from `templates/article.html`. |
| `http://localhost:3000/pricing` | `GET` | **Capital** | SaaS pricing tiers, `Billable` quota check & SPED DPS XMLDSig. |
| `http://localhost:3000/security-demo` | `GET` | **Security** | Interactive RASP, Anti-Timing Guard & AI Firewall sandbox. |
| `http://localhost:3000/ai-assistant` | `GET` | **AI & RAG** | Vector semantic search with Cosine Similarity & Prompt Shield. |
| `http://localhost:3000/wp-admin` | `GET` | **Honeypot** | Trap route triggering threat log to SOC Threat Radar. |
| `http://localhost:3000/studio` | `GET` | **Studio** | Developer Control Room (Database Inspector, Radar, Traces). |
| `http://localhost:3000/nexus` | `GET` | **Nexus** | Admin CMS with CRUD management and AI Assistant. |
| `http://localhost:3000/robots.txt` | `GET` | **SEO** | Auto-generated crawler directives. |
| `http://localhost:3000/sitemap.xml` | `GET` | **SEO** | XML sitemap metadata. |

---

## 🚀 Running Locally

```bash
# From workspace root
cargo run -p rullst-blog-example

# Or from this directory
cargo run
```

Open `http://127.0.0.1:3000` in your browser.

---

## 🏢 Multi-Tenant Scoping Test

By default, the database initializes isolated records for different tenants:

```bash
# 1. Enterprise Tenant:
curl -s -H "X-Tenant-ID: tenant-enterprise" http://localhost:3000/ | grep "Enterprise Architecture"

# 2. Startup Tenant:
curl -s -H "X-Tenant-ID: tenant-startup" http://localhost:3000/ | grep "High-Velocity MVP"

# 3. Community Tenant (Default):
curl -s http://localhost:3000/ | grep "The Sovereign SaaS Blog"
```

---

## 🧪 CI/CD & Automated Testing

This application is verified in GitHub Actions via:
- [`.github/workflows/e2e-smoke.yml`](../../.github/workflows/e2e-smoke.yml): Builds release binary, validates SSR status 200, CSP security headers, CSRF tokens, and SQLite writes.
- [`.github/workflows/dast-zap.yml`](../../.github/workflows/dast-zap.yml): Runs OWASP ZAP dynamic vulnerability scanner against the live running instance.
- [`.github/workflows/coverage.yml`](../../.github/workflows/coverage.yml): Validates code coverage exceeds 80% under `cargo-llvm-cov`.
