# Rullst Roadmap 🗺️
### *"The Path to the Ultimate Full-Stack Rust Framework"*

*Read this in [Português (Brasil)](./ROADMAP.pt.md).*

This roadmap outlines the milestones required to transition **Rullst** from its current MVP (v0.1.0) into a dominant, production-ready, full-stack framework focused on **Emotional Productivity** and **AI-Native Engineering**.

Our development strategy follows the **"Developer Experience like Laravel, Performance like Rust, Architected for Humans and AI"** philosophy.

---

## 🤖 The AI-Native Paradigm (Designed for Humans & AI)

Almost every modern web framework (Laravel, Ruby on Rails, Next.js) was built before the era of LLMs and AI Agents. They rely heavily on runtime magic, dynamic reflection, and complex implicitness that confuses AI coders and leads to hallucinations.

**Rullst is built from the ground up to be the first AI-Native web framework:**
1. **Zero Runtime Magic, Pure Compilation:** High-level declarative macros (`#[derive(Eloquent)]`, `routes!`) and strict Rust type safety give AI coding assistants extremely explicit structures, resulting in zero API hallucinations and instant compiler self-correction.
2. **Context-Rich Scaffolding:** `cargo rullst new` will automatically scaffold optimized `.ai-rules` / `.cursorrules` files. Any AI agent opening the project instantly learns Rullst's exact conventions, code style, and API standards, achieving 100% productive pair-programming immediately.
3. **Structured System Discovery:** In dev mode, Rullst will generate a local structural schema (`rullst-schema.json`) detailing all active routes, controllers, and models. This lets AI agents map out the entire project structure in milliseconds.

---

## 🚀 The Rullst Master Plan

```mermaid
graph TD
    M0[Pillar: AI-Native Design] --> M1[Milestone 1: CLI Generator Power]
    M1 --> M2[Milestone 2: Database Supremacy]
    M2 --> M3[Milestone 3: Complete Auth & Security]
    M3 --> M4[Milestone 4: HTMX & Frontend Integration]
    M4 --> M5[Milestone 5: Production Utilities]
    style M0 fill:#ffecd2,stroke:#ff9a00,stroke-width:3px,color:#000
    style M1 fill:#00f2fe,stroke:#fff,stroke-width:2px,color:#000
    style M2 fill:#4facfe,stroke:#fff,stroke-width:2px,color:#000
    style M3 fill:#a18cd1,stroke:#fff,stroke-width:2px,color:#000
    style M4 fill:#fbc2eb,stroke:#fff,stroke-width:2px,color:#000
    style M5 fill:#ff9a9e,stroke:#fff,stroke-width:2px,color:#000
```

---

## 🛠️ Milestone 1: CLI Empowerment (`cargo-rullst`)
**Goal:** Enable lightning-fast scaffolding. Developers should never create boilerplate files manually.

- [ ] **Code Generators:**
  - [ ] `cargo rullst make:controller <Name>` - Generates a controller with standard CRUD actions.
  - [ ] `cargo rullst make:model <Name> [-m]` - Generates an Active Record model and optionally an associated migration.
  - [ ] `cargo rullst make:middleware <Name>` - Generates Axum-compatible custom middleware.
- [ ] **Workspace Ergonomics:**
  - [ ] Improve compilation speeds for CLI runs.
  - [ ] Support `--api` flag for scaffolding headless REST APIs instead of full HTML apps.

---

## 🗄️ Milestone 2: Database Supremacy (Migrations & Relationships)
**Goal:** Empower `rust-eloquent` and `Rullst` to handle enterprise-grade relational schemas seamlessly.

> [!NOTE]
> **Division of Responsibilities:**
> The heavy lifting (database schema parsers, migration execution, and relationship macro builders) will be developed directly inside the **`rust-eloquent`** repository to keep the ORM modular.
> **Rullst** will wrap these features with CLI commands and smooth dependency injection.

- [ ] **Migration Engine (in `rust-eloquent`):**
  - [ ] SQL-based or DSL-based migration definitions.
  - [ ] CLI runner inside Rullst:
    - [ ] `cargo rullst db:migrate` - Runs pending migrations.
    - [ ] `cargo rullst db:rollback` - Reverts the last migration batch.
    - [ ] `cargo rullst db:status` - Shows the migration history.
- [ ] **Active Record Relationships (in `rust-eloquent`):**
  - [ ] `HasMany` / `BelongsTo` declarative macros.
  - [ ] `BelongsToMany` (Many-to-Many) association resolvers.
  - [ ] Lazy and Eager loading mechanisms to prevent N+1 query problems.
- [ ] **Seeders and Factories:**
  - [ ] `cargo rullst db:seed` - Populate databases using pre-configured mock data.
  - [ ] Integrated factory pattern for mock entity generation.

---

## 🔒 Milestone 3: Authentication & Security (Social & Local Auth)
**Goal:** Implement robust, secure, and instant authentication. Developers should be able to authenticate users securely in minutes.

- [ ] **Social Authentication via `rust-socialite`:**
  - [ ] Leverage the custom **[`rust-socialite`](https://crates.io/crates/rust-socialite)** crate as the official OAuth engine.
  - [ ] Out-of-the-box configurations for Google, GitHub, Facebook, Twitter, and custom providers.
  - [ ] Seamless flow: redirect to provider, parse callbacks, and login/register users via Active Record.
- [ ] **Local Authentication:**
  - [ ] Secure password hashing via Argon2/Bcrypt built-in helpers.
  - [ ] Custom session-based cookie middleware and token-based (JWT) auth middleware.
- [ ] **The "Auth Magic" Command:**
  - [ ] `cargo rullst auth` - Instantly scaffold a full-fledged authentication system containing:
    - Login/Registration/Password Reset controllers.
    - Beautiful UI screens (`html!` templates) pre-configured with CSS.
    - SQL database migration for the `users` table.
- [ ] **Security Defaults:**
  - [ ] Automatic CSRF protection for HTML form submissions.
  - [ ] Default security headers middleware (CORS, HSTS, X-Content-Type-Options).

---

## ⚡ Milestone 4: HTMX & Interactivity
**Goal:** Combine the simplicity of Server-Side Rendering (SSR) with the snappy feeling of modern Single-Page Applications (SPAs).

- [ ] **HTMX First-Class Support:**
  - [ ] Built-in response helpers for checking HTMX headers (`rullst::htmx::is_htmx(req)`).
  - [ ] Native support for partial template rendering (rendering only the requested component, not the full page layout).
  - [ ] TailwindCSS auto-integration during project setup.

---

## 📦 Milestone 5: Production Utilities (Queues, Cache, Scheduler)
**Goal:** Provide the tools needed to scale applications in production environment.

- [ ] **Queues & Background Workers:**
  - [ ] `rullst::queue` API supporting SQLite (for local dev) and Redis (for production).
  - [ ] Asynchronous task workers executing jobs in the background.
- [ ] **Caching Layer:**
  - [ ] `rullst::cache` unified driver API supporting In-Memory and Redis adapters.
- [ ] **Task Scheduler:**
  - [ ] Declarative Cron-like job scheduler directly in `main.rs` (e.g. `.schedule("0 0 * * *", nightly_cleanup)`).

---

## 🗺️ Execution Strategy

We will proceed **milestone by milestone**, starting with **Milestone 1** to polish our CLI generators. 

If you are ready to begin, select a task or suggest which component to build next! 🚀
