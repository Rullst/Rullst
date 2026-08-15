# 🏗️ Architecture Choices: ORM Patterns & Frontend Engines

When initializing a new application using `cargo rullst new`, the Rullst CLI prompts you to select two fundamental architectural pillars:
1. **ORM Pattern / Architecture** (Active Record vs. Data Mapper / Repository vs. Hybrid)
2. **Frontend Engine** (Zero-Bundle HTMX vs. Leptos SSR vs. Dioxus SSR)

This guide provides a detailed breakdown of pros, cons, maintenance trade-offs, and clear guidelines on when to pick each option.

---

## 🗄️ Part 1: ORM Patterns Breakdown

Rullst supports three distinct database access patterns via `rullst-orm`.

### 1. Active Record Mode (`User::find(id)`)
In Active Record, your database models inherit both data properties and database access methods. The model itself knows how to insert, update, query, and delete its own row.

* **Pros:**
  * **Extreme Velocity:** 50% less boilerplate code. You don't need to write separate repository interfaces or DTO mappers for standard CRUD operations.
  * **Intuitive API:** Expressive methods like `User::find(1)`, `user.update()`, `User::where("status", "active")`.
  * **Ideal for 90% of Projects:** Perfect for SaaS platforms, REST APIs, e-commerce stores, blogs, and rapid prototyping.
* **Cons:**
  * Couples domain entities directly with database table schemas.
* **When to Pick:** Recommended for almost all new projects, startups, MVPs, and standard business applications.

---

### 2. Data Mapper / Repository Pattern (`UserRepository::find(id)`)
In Data Mapper, your domain structs are pure Rust data containers without any database access logic. A separate `Repository` struct manages all SQL queries, transactions, and database mapping.

* **Pros:**
  * **Strict Decoupling (DDD):** Domain entities are completely decoupled from database mechanics.
  * **Enterprise Compliance:** Fits strict Domain-Driven Design (DDD) and Clean Architecture rules required by large financial institutions or legacy database migrations.
* **Cons:**
  * Requires writing and maintaining duplicate structs (Domain Struct + Database Table DTO + Repository Implementation).
* **When to Pick:** Large enterprise systems, banking/fintech platforms with complex domain logic rules, or legacy databases where table structures differ drastically from business objects.

---

### 3. Hybrid Architecture (Active Record + Repository)
Enables both Active Record traits and Repository pattern traits in your codebase.

* **Important Note:** Hybrid mode is **not a runtime failover** (i.e. if one fails, the other does not act as an automatic backup). In Rust, both patterns are type-checked at compile time. Hybrid simply means your application can freely mix both paradigms.
* **Pros:**
  * **Flexibility:** Use Active Record for simple tables (Categories, Logs, Tags, Profiles) while using Repository pattern for complex domain modules (Payment Gateways, Ledger Accounting).
* **Cons:**
  * **Inconsistent Team Conventions:** Without strict team rules, Developer A might write Active Record (`user.save()`) while Developer B writes Repository code (`user_repo.save(&user)`) for the exact same entity, leading to mixed coding styles across the project.
  * **Higher Cognitive Overhead:** Team members must learn and maintain two database paradigms simultaneously.
* **When to Pick:** Mid-to-large teams with clear guidelines on which modules use Active Record vs. Repository.

---

### ❓ "If I choose Active Record now, will I be locked in when my app grows?"

**No! Rullst guarantees zero architectural lock-in.**

If you start your project with **Active Record**, you write fast, clean CRUD code today. Two years from now, if your startup grows and you need to introduce a complex financial ledger requiring the Repository pattern:
1. You **do NOT need to rewrite** your existing Active Record models.
2. You can simply create a `Repository` struct for the new financial module alongside your existing code.
3. Rullst seamlessly allows you to adopt Repository pattern features in specific files whenever you need them.

#### ORM Pattern Quick Comparison

| Feature | Active Record | Data Mapper / Repository | Hybrid Architecture |
| :--- | :--- | :--- | :--- |
| **Development Velocity** | ⚡ **Fastest (50% less code)** | 🐢 Slower (More boilerplate) | ⚡ Moderate |
| **Code Uniformity** | 🎯 **100% Consistent** | 🎯 **100% Consistent** | ⚠️ Can become mixed |
| **Learning Curve** | 🟢 Very Easy | 🔴 Steeper (DDD concepts) | 🟡 Moderate |
| **Decoupling from SQL** | 🟡 Moderate | 🟢 **100% Decoupled** | 🟢 Flexible per module |
| **Future Upgrade Path** | 🟢 Easily add Repositories later | 🟢 Already Enterprise | 🟢 Already Both |

---

## 🎨 Part 2: Frontend Engines Breakdown

Rullst allows you to choose your rendering strategy based on user experience goals, bundle constraints, and team expertise.

| Frontend Engine | JS Bundle Size | SSR Strategy | Interactivity Model | Framework Reference Pattern | Target Use Cases |
| :--- | :---: | :--- | :--- | :--- | :--- |
| **1. Zero-Bundle HTMX + Tailwind** *(Default)* | **0 KB** | HTML5 Server-Driven | HTMX Attributes + `html!` | **HTMX Standard** | Web Apps, SaaS, Dashboards, E-commerce |
| **2. LiveView Server-Driven UI** | **0 KB** | Server-Side State Machine | Persistent Tokio WebSockets | **Phoenix & Dioxus Live** | Real-time Feeds, Chat, Interactive Collaboration |
| **3. Reactive Wasm Islands** | **Pontual** | Client WebAssembly VM | Fine-Grained Signals | **Leptos & Yew WASM** | Canvas, Markdown Editors, Offline Rich UIs |
| **4. Zero-Build Pure CSS (Topcoat UI)** | **0 KB** | Pure CSS Server-Rendered | 60 FPS GPU Hardware Accel | **Adobe Topcoat CSS** | Backend Tools, Dashboards (0 Node.js / 0 NPM) |
| **5. File-Based Classic Templates (Tera)** | **0 KB** | File Templates (`templates/*.html`) | Server-Rendered HTML | **Loco.rs, Rails & Django** | Traditional MVC monoliths with external HTML |

---

### 1. Zero-Bundle HTMX + TailwindCSS *(Recommended Default)*
Rullst renders semantic HTML5 on the server at compile time using the `rullst::html!` macro and streams tiny, targeted HTML swaps over the wire via HTMX.

* **Pros:**
  * **Sub-10ms Page Loads:** No heavy JavaScript bundles to parse or execute on the client.
  * **Maximum SEO & Accessibility:** Clean semantic HTML rendered instantly.
  * **Simpler Maintenance:** 100% of business logic stays on the server in type-safe Rust.
* **When to Pick:** Recommended for 90% of web applications, SaaS platforms, portals, and e-commerce stores.

---

### 2. LiveView Server-Driven UI (`rullst::live`)
State machine running directly in Tokio server memory with bidirectional WebSocket diff patching to the browser (the Phoenix LiveView & Dioxus Live pattern).

* **Pros:** Real-time reactivity without writing a single line of client JavaScript.
* **When to Pick:** Live chat rooms, collaborative editing, real-time telemetry counters, and active IoT sensor dashboards.

---

### 3. Reactive Wasm Islands (`rullst::island` / `#[client_component]`)
Client-side WebAssembly micro-frontends compiled via `wasm-bindgen` and mounted pontually on specific pages (the Leptos & Yew WASM/Signals pattern).

* **Pros:** True in-browser high-performance computing without VDOM overhead.
* **When to Pick:** Highly dynamic client-side web tools (e.g. browser Markdown editors, canvas games, offline calculators, interactive charting).

---

### 4. Zero-Build Pure CSS (Topcoat UI)
Ultra-high performance pure CSS component kit created by Adobe Web Platform benchmarked for 60 FPS performance with zero JavaScript runtime and zero Node.js/NPM build tools.

* **Pros:** Instant compiling with pure `cargo run`, zero npm dependencies, beautiful dark-mode native components out of the box.
* **When to Pick:** Backend engineers and DevOps developers wanting clean, fast web dashboards without setting up JavaScript bundlers.

---

### 5. File-Based Classic Templates (Jinja2 / Tera Engine)
Decoupled HTML templates stored in a dedicated `templates/` directory with full layout inheritance (`{% extends "base.html" %}`).

* **Pros:** Designers can modify HTML template files directly without touching Rust source code or triggering Rust compiler recompilations.
* **When to Pick:** Teams migrating from Django, Ruby on Rails, Laravel, or Loco.rs.

---

### 🏛️ Scope: Application Site vs. Built-in Admin Tools (Nexus & Studio)

When you select a Frontend Engine for your project:

1. **Application Site Routes (`/`, `/courses`, `/posts`, etc.)**: Render using your chosen Frontend Engine (HTMX, LiveView, Wasm Island, Topcoat, or Tera).
2. **Rullst Studio (`/studio` at `:5555`) & Rullst Nexus (`/nexus`)**: Are embedded developer and administrative control rooms provided directly by the framework kernel (`rullst-studio` and `rullst-nexus`). They are pre-compiled with an ultra-lightweight, zero-bundle HTMX + dark glassmorphic interface.

> **Key Benefit:** Regardless of the frontend engine chosen for your application, your administrative panel (`/nexus`) and telemetry dashboard (`/studio`) load instantly with zero client-side JavaScript bundle overhead!

---

### 🌐 Project Types: Full-Stack Web App vs. Headless REST API

During project creation (`cargo rullst new`), you can choose between two main application modes:

1. **Full-Stack Web App**:
   - Generates server-side rendered pages using your chosen Frontend Engine.
   - Prompts for Frontend Engine selection during wizard setup.
   - Ideal for SaaS platforms, portfolios, e-commerce, blogs, and administrative systems.

2. **Headless REST API**:
   - Generates lightweight JSON endpoints (`rullst::server::Json(payload)`).
   - Automatically skips Frontend Engine selection to keep dependencies clean and minimal.
   - Ideal for microservices, mobile app backends, and headless APIs.

### 🛠️ Developer Tools & Admin Panels (Studio & Nexus Availability)

- **Rullst Visual Studio (`http://127.0.0.1:5555`)**: Launched automatically in development mode across **all** project types (Full-Stack and Headless REST API). Provides real-time Database Inspector, RASP Security Radar, and Tokio Telemetry Spans.
- **Rullst TUI Dashboard (`cargo rullst dash`)**: Interactive CLI terminal dashboard monitoring HMR builds, HTTP request logs, and dev shortcuts across all project types.
- **Rullst Nexus Auto-CMS (`/nexus`)**: Pre-configured out of the box in starter blueprints (**Portfolio**, **SaaS**, **LMS**, **Blog**, **ERP**). For **Blank Starter** or custom API projects, the `nexus` feature is enabled in `Cargo.toml` whenever a database is present, allowing you to register custom models via `.nest_axum("/nexus", nexus)` at any time.

---

### 4. Integration with Rullst Omni (`cargo rullst make:omni`)

#### Clarifying Frontend Engines vs. Rullst Omni (Tauri 2.0 Integration)
* **Rullst Omni (`cargo rullst make:omni` / Tauri 2.0):** Acts as the **Native Operating System Shell & Container**. It creates native OS windows (`.exe`, `.app`, `.apk`, `.ipa`), handles system tray icons, native OS notifications, auto-updates, and low-level system APIs.
* **Frontend Engine (HTMX / LiveView / Wasm / Topcoat):** Acts as the **UI Engine / Renderer** running inside the Omni webview window with zero friction.

---

## 🤖 Part 3: AI Engine Selection (`rullst-ai`)

During project setup, Rullst asks whether to enable native AI capabilities (`rullst-ai`).

### Option A: Enable `rullst-ai` (`Yes`)
Includes the `rullst-ai` crate with native multi-provider LLM support (OpenAI, Anthropic, Gemini, local Ollama), Vector Index (`CosineSimilarity`), RAG Prompt builder (`build_rag_prompt`), and Agentic Tool Registry (`ToolRegistry`).

* **Pros:**
  * **Instant AI Integration:** Out-of-the-box support for RAG, vector embeddings, local LLM execution, stateful chat sessions (`cargo rullst make:chat-session`), and Function Calling tools (`AiTool`).
  * **Zero Extra Boilerplate:** Native Rust builders for all major LLM APIs without managing raw HTTP requests or custom JSON serializers.
* **Cons:**
  * Increases initial binary size by ~2-3MB and adds a small overhead to compilation time.
* **When to Pick:** Applications building AI features (RAG search, AI chatbots, automated document summaries, local LLM agents).

### Option B: Disable `rullst-ai` (`No`)
Omits `rullst-ai` from your dependencies.

* **Pros:**
  * **Slimmer Binary & Lightning Compilation:** Minimal dependency graph for ultra-fast builds and tiny production binaries.
* **Cons:**
  * Requires adding `rullst-ai` manually later if you decide to integrate LLMs.
* **When to Pick:** Traditional Web APIs, CRUD applications, SaaS backends, microservices, or embedded systems that don't need AI capabilities.

---

## ⚡ Part 4: Redis & Cache Architecture (`rullst::cache`)

Rullst offers a choice between Distributed Redis Caching or In-Memory Local Caching.

### Option A: Enable Redis Caching & Queues (`Yes`)
Connects Rullst to a Redis instance (`rullst::cache::redis`) for distributed caching, persistent background job queues, and multi-node WebSockets.

* **Pros:**
  * **Horizontal Scaling:** Cache data, user sessions, and rate limits are shared seamlessly across multiple server nodes/replicas.
  * **Persistent Queues & Pub/Sub:** Background workers (`make:worker`) survive server restarts and scale across worker pools; WebSockets sync across nodes.
* **Cons:**
  * Requires running and managing a Redis instance (or a managed service like AWS ElastiCache / Upstash).
* **When to Pick:** Production applications with multiple server replicas, heavy background job processing, distributed user sessions, or horizontal WebSocket scaling.

### Option B: In-Memory Cache & Local Tasks (`No`)
Uses thread-safe in-memory caching (`rullst::cache::memory` via DashMap) and Tokio channels for background tasks.

* **Pros:**
  * **Zero Infrastructure Overhead:** No Redis server required. Everything runs inside the single Rust binary executable.
  * **Sub-Microsecond Latency:** Cache reads happen directly in RAM without network roundtrips.
* **Cons:**
  * Cache data and background queues are cleared if the server restarts.
  * Single-instance only (cannot share cache state across multiple independent server nodes).
* **When to Pick:** Single-server deployments, side projects, MVPs, embedded IoT nodes, or applications where in-memory caching is sufficient.

---

## 🛡️ Built-in Zero-Config Pillars: Security & Telemetry

You might wonder: *Why doesn't `cargo rullst new` ask whether to include Security (`rullst-security`) or Telemetry (`rullst::radar`)?*

### The DX Principle: "Security & Observability by Default"

Rullst adheres to a strict Developer Experience (DX) philosophy: **Essential protection and real-time observability must never be opt-in choices.**

1. **`rullst-security` (RASP, Vault, Honeypots, Threat Radar):** 
   * **Why Built-in:** Every Rullst app automatically inherits Runtime Application Self-Protection (RASP), zero-trust field encryption (`Zeroize`), honeypot traps, and threat analysis. Leaving security as a CLI prompt risks developers accidentally creating vulnerable applications.
   * **Zero Overhead:** Security checks run as compiled, zero-cost static middleware with `< 50KB` RAM footprint.

2. **`rullst::radar` (Kernel Telemetry & Prometheus `/metrics`):**
   * **Why Built-in:** Real-time RSS memory tracking, Tokio event loop tick latency measurement, and `/metrics` exporters are automatically initialized in every Rullst app. This ensures instant observability without setting up third-party agents.

By keeping Security and Telemetry **built-in by default**, the CLI avoids "survey fatigue" (asking 15+ questions) and limits prompts strictly to structural choices that impact external infrastructure or cloud costs (Database Engine, ORM Pattern, Frontend Framework, AI Module, Redis).

---

## 💡 Summary Recommendations

1. **For most web applications:** Choose **Active Record Mode** + **Zero-Bundle HTMX** + **No AI** + **In-Memory Cache**. This gives you maximum speed, zero JS overhead, and minimal infrastructure complexity.
2. **For AI-powered applications:** Choose **`rullst-ai` Enabled (`Yes`)** for instant LLM, RAG, and Tool-Calling support.
3. **For multi-node production apps:** Choose **Redis Enabled (`Yes`)** for horizontal cache, queue, and WebSocket scaling.
4. **For multi-platform desktop/mobile apps:** Pair any frontend engine (**HTMX**, **Leptos**, or **Dioxus**) with **Rullst Omni** (`cargo rullst make:omni`).
