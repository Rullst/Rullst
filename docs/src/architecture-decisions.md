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

Rullst allows you to choose your rendering strategy based on user experience goals and team expertise.

| Frontend Engine | JS Bundle Size | SSR Strategy | Interactivity Model | Target Use Cases |
| :--- | :--- | :--- | :--- | :--- |
| **Zero-Bundle HTMX + TailwindCSS** | **0 KB** | HTML5 Server-Driven | HTMX Attributes + LiveView | Web Apps, SaaS, Dashboards, E-commerce |
| **Leptos SSR Adapter** | ~150 KB (Wasm) | Server-Side + Client Hydration | Rust Signals & Wasm | Complex Web UIs, Canvas, Web Games |
| **Dioxus SSR Adapter** | ~180 KB (Wasm) | Server-Side + Client Hydration | Virtual DOM & Wasm | Multi-Platform (Web + Desktop + Mobile) |

---

### 1. Zero-Bundle HTMX + TailwindCSS *(Recommended Default)*
Rullst renders semantic HTML5 on the server and streams tiny, targeted HTML swaps over the wire via HTMX and WebSockets (`rullst::live`).

* **Pros:**
  * **Sub-10ms Page Loads:** No heavy JavaScript bundles to parse or execute on the client.
  * **Maximum SEO & Accessibility:** Clean semantic HTML rendered instantly.
  * **Simpler Maintenance:** 100% of business logic stays on the server in type-safe Rust.
* **When to Pick:** Recommended for 99% of web applications, SaaS platforms, portals, and e-commerce stores.

---

### 2. Leptos SSR Adapter
Full-stack reactive Rust framework utilizing fine-grained signals and WebAssembly client hydration.

* **Pros:** React-like reactive DX using pure Rust signals (`create_signal`).
* **Cons:** Larger compile times and initial Wasm bundle downloads.
* **When to Pick:** Highly dynamic client-side web tools (e.g. browser image editors, interactive charts, WebAssembly widgets).

### 3. Dioxus SSR Adapter
Full-stack cross-platform Rust UI framework utilizing Virtual DOM and RSX macro syntax.

* **Pros:**
  * **React-Like Developer Experience:** Familiar `rsx!` macro syntax, hooks (`use_signal`), and component-based state.
  * **Cross-Platform Reusability:** Share UI components across Web (Wasm), Desktop, and Mobile without rewriting component state.
* **Cons:**
  * Uses a Virtual DOM (VDOM) diffing engine, adding slight runtime overhead compared to fine-grained signal frameworks.
* **When to Pick:** Teams coming from React/JSX who want a declarative Virtual DOM UI engine for multi-platform Web, Desktop, and Mobile applications.

---

### 4. Integration with Rullst Omni (`cargo rullst make:omni`)

#### Clarifying Frontend Engines vs. Rullst Omni (Tauri 2.0 Integration)
A common question arises: *If Dioxus supports desktop/mobile apps, how does it fit with Rullst Omni (which uses Tauri)? Do they conflict?*

**The Answer: They integrate seamlessly and complement each other!**

* **Rullst Omni (`cargo rullst make:omni` / Tauri 2.0):** Acts as the **Native Operating System Shell & Container**. It creates native OS windows (`.exe`, `.app`, `.apk`, `.ipa`), handles system tray icons, native OS notifications, auto-updates, and low-level system APIs.
* **Dioxus / HTMX / Leptos:** Acts as the **UI Engine / Renderer** running inside the Omni webview window.

#### How They Work Together:
1. **Dioxus as the UI Layer:** Renders Virtual DOM components in Rust with smooth animations and state management.
2. **Omni (Tauri) as the System Layer:** Exposes native OS capabilities (e.g. `omni::fs::read_file()`, native Bluetooth, system tray menus).

When you build a desktop/mobile app with Rullst:
- **HTMX + Rullst Omni:** Rullst renders fast server-driven UI inside the Tauri window with 0KB JS overhead.
- **Leptos + Rullst Omni:** Leptos renders fine-grained reactive signals & Wasm UI inside the Tauri window with ultra-fast DOM updates.
- **Dioxus + Rullst Omni:** Dioxus renders Virtual DOM UI inside the Tauri window with smooth cross-platform component state.

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
