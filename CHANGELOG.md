# Changelog 📝

All notable changes to the **Rullst Framework** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.7.0] - 2026-05-25 🤖

### Added (AI-Native Core Milestone)
- **Extensible AI Facade (`rullst::ai`):**
  - Introduced the `AiClient` facade and the `AiProvider` trait (similar to Rullst Storage and Mailer patterns) to build highly extensible AI applications.
  - Implemented automatic driver resolution via `AiClient::auto()`, which dynamically detects `OPENAI_API_KEY`, `GEMINI_API_KEY`, `ANTHROPIC_API_KEY`, or `OLLAMA_HOST` from environment variables.
- **Multi-Provider Drivers (`rullst::ai::providers`):**
  - `OpenAiProvider`: Integrates with OpenAI models (e.g. `gpt-4o-mini`) and text embeddings.
  - `GeminiProvider`: Full integration with Google Gemini models (e.g. `gemini-1.5-flash`), with native support for `systemInstruction` parameters.
  - `AnthropicProvider`: Claude integration utilizing the Messages API and top-level system prompts.
  - `OllamaProvider`: Local LLM execution supporting local completions (e.g. `llama3`) and vector embeddings (e.g. `nomic-embed-text`) via Ollama.
- **Fluent Chat Builder (`ChatBuilder`):**
  - Fluent builder for multi-turn conversational agents with simple `.system()`, `.user()`, and `.assistant()` methods.
  - Handles dynamic role mapping per provider transparently (e.g., mapping `assistant` role to `model` role in Gemini).
- **Strongly Typed Structured Prompts:**
  - Added `structured_prompt<T>` helper to parse LLM outputs into strongly typed Rust structs, automatically sanitizing markdown wraps (e.g., ` ```json ... ``` `).
- **In-Memory RAG Engine (`VectorIndex`):**
  - Zero-dependency, pure Rust in-memory `VectorIndex` for instant vector search.
  - Utilizes high-performance Cosine Similarity algorithms to let developers build light, instant RAG applications without external vector databases.

## [0.6.1] - 2026-05-25 🛠️

### Added (CLI Empowerment & Generators completions)
- **Interactive Project Scaffolding (`cargo rullst new`):**
  - Added a beautiful prompt-based wizard wizard asking for App Name, App Type (Fullstack SSR vs REST API), and Database Provider (SQLite, PostgreSQL, MySQL) using the `dialoguer` crate.
  - Automatically structures dependencies, configuration database connection strings (`Rullst.toml`), and generated boilerplate templates based on wizard choices.
- **Milestone 1 CLI Generators:**
  - `make:cors`: Generates a standard Axum CORS middleware in `src/middlewares/cors_middleware.rs` with OPTIONS preflight handling and safe owned string lifetime parameters.
  - `make:jwt`: Generates a token-based JWT authentication middleware in `src/middlewares/jwt_middleware.rs` with a `generate_token` helper, injecting `jsonwebtoken` and `chrono` into `Cargo.toml`.
  - `make:worker`: Generates background task worker modules and registers them inside `src/workers/mod.rs` for processing asynchronous queue tasks.
  - `generate:openapi`: Zero-magic static analysis OpenAPI generator that scans `src/main.rs` route patterns and `src/controllers/` actions' doc-comments (`///`) to output a high-performance `openapi.json` spec.

## [0.6.0] - 2026-05-25 🏢

### Added (Enterprise Features Milestone)
- **Declarative Validation (`rullst::validation`):**
  - Added `ValidatedForm<T>` and `ValidatedJson<T>` Axum extractors that automatically perform validations using the `validator` crate.
  - Generates beautiful HTMX validation error lists for frontend clients, or redirects, or returns standard `422 Unprocessable Entity` JSON responses automatically based on client negotiation.
- **Mailer System (`rullst::mail`):**
  - Added unified `Mail` facade and `MailDriver` trait.
  - Implemented `LogDriver` for local development, `SmtpDriver` for classic email setups, and highly optimized, async REST-based `ResendDriver` and `SendGridDriver` utilizing `reqwest` and `rustls` (zero-openssl dependency for maximum factory productivity).
- **Storage Abstraction (`rullst::storage`):**
  - Unified `Storage` facade and `StorageDriver` trait.
  - Implemented `LocalDriver` writing files locally under `storage/app`, and AWS-compliant `S3Driver` for cloud storage.
- **WebSockets & Real-Time (`rullst::ws`):**
  - High-level `WebSocket` wrapper for real-time messaging.
  - Seamlessly integrated with Axum, supporting high-level HTMX out-of-band swaps via `.send_html()`.
  - Added `.ws(path, handler)` and `.nest` routing methods to Rullst `Router` for modular setups.
- **Rullst Horizon (`rullst::horizon`):**
  - Gorgeous, premium, high-fidelity dark mode dashboard built entirely in Rust using raw `html!` templates and HTMX polling.
  - Real-time queue metrics (pending counts, failed jobs, active worker status), failed jobs detail lists, and instant one-click dashboard retries/purges!

---

## [0.5.0] - 2026-05-25 📦

### Added (Production Utilities Milestone)
- **Docker & Containerization (`cargo rullst new --docker`):**
  - Multi-stage `Dockerfile` using `rust:1.87-slim` builder → `gcr.io/distroless/cc-debian12` runtime (~20MB final image).
  - Auto-generated `docker-compose.yml` with App + PostgreSQL 16 + Redis 7 services, health checks, and persistent volumes.
  - `.dockerignore` to exclude build artifacts and dev files.
- **Queue & Background Workers (`rullst::queue`):**
  - `Queue` facade with `dispatch()` for pushing named jobs with JSON payloads.
  - `Worker` with `register()` for mapping job names to async handler closures and `run()` for background processing.
  - `SqliteDriver`: Uses auto-created `rullst_jobs` table, zero config, FIFO with atomic pop.
  - `RedisDriver` (optional, `queue-redis` feature): Uses Redis lists for high-throughput distributed workloads.
- **Caching Layer (`rullst::cache`):**
  - `Cache` facade with `get`/`put`/`forget`/`flush`/`has` and the `remember()` cache-aside pattern.
  - `MemoryDriver`: Lock-free `DashMap`-based concurrent store with lazy TTL expiration.
  - `RedisDriver` (optional, `cache-redis` feature): Redis-backed with `SETEX` TTL support and `rullst:cache:` key prefix.
- **Task Scheduler (`rullst::scheduler`):**
  - `Scheduler` with `.task("cron_expr", handler)` for registering recurring async jobs.
  - Standard 5-field cron expressions auto-converted to 7-field for the `cron` crate.
  - Integrated into `Server` via `.schedule(scheduler)` builder method — runs alongside HTTP server.

---

## [0.4.0] - 2026-05-25 ⚡

### Added (HTMX & Interactivity Milestone)
- **HTMX First-Class Support (`rullst::htmx`):**
  - Added `HtmxRequest` extractor to easily detect `HX-Request` and other HTMX headers in Axum routes.
  - Added `HtmxResponse` builder for setting HTMX-specific response headers (like `HX-Trigger`, `HX-Redirect`, `HX-Retarget`).
  - Added `render_page` macro/helper for hybrid SSR rendering, automatically serving partial fragments for HTMX requests or the full HTML layout for standard browser visits.
- **TailwindCSS Integration:**
  - `cargo rullst new` now automatically configures TailwindCSS via CDN in the generated templates.
  - Scaffolded projects include a reactive HTMX counter component to demonstrate immediate interactivity without writing JavaScript.
- **Hyphenated HTML Attributes (`rullst-macros`):**
  - Updated the `html!` procedural macro to fully support hyphenated attributes like `hx-post`, `hx-target`, and `hx-swap`.

---

## [0.3.0] - 2026-05-25 🛡️

### Added (Authentication & Security Milestone)
- **Local Authentication Primitives (`rullst::auth`):**
  - High-security password hashing and verification powered by **Argon2id**.
  - Secure **AES-256-GCM** client-side encrypted cookie sessions (`rullst_session`) valid for 30 days.
  - Automatic `APP_KEY` cryptographic key resolution from environment variables or `Rullst.toml`.
- **Double Submit CSRF Validation (`rullst::security::csrf_middleware`):**
  - Automatic injection of secure CSRF cookies on GET requests.
  - Validation of state-modifying requests (`POST`, `PUT`, `DELETE`) comparing cookie tokens with HTTP headers (`X-CSRF-Token`) or hidden `_token` fields.
  - Custom stream re-builder to safely buffer the request body during verification.
- **Production Security Headers (`rullst::security::headers_middleware`):**
  - Standard headers injected on all HTTP responses: HSTS, Content-Type-Options (nosniff), Frame-Options (DENY), XSS-Protection, and Referrer-Policy.
- ** CLI Auth Command (`cargo rullst auth`):**
  - Scaffold entire authentication systems (local register, login, logout, and GitHub social auth redirect and callback handlers via the dynamic `rust-socialite` sibling dependency).
  - Scaffold database migrations for `users`, the `User` Active Record model, and restricted route `AuthMiddleware`.
  - Scaffold beautiful responsive Dark Mode HTML templates (`login_page`, `register_page`, `dashboard_page`) using the procedurally compiled `html!` macro.

---

## [0.2.0] - 2026-05-25 🚀

### Added (Database Supremacy Milestone)
- **Artisan CLI Engine (`rullst::artisan!`):** A declarative macro that intercepts process execution to run database migrations, seeds, and status checks directly within the application binary before the server boots.
- **Rullst Dev CLI Migrations:** `cargo-rullst` now proxies artisan commands (`db:migrate`, `db:rollback`, `db:status`, `db:seed`) gracefully to the target workspace.
- **Database agnostic URL Injection:** Rullst `Server::new` now auto-parses `Rullst.toml` and automatically injects the `DATABASE_URL` into the `rust-eloquent` connection pool during boot, supporting SQLite, PostgreSQL, and MySQL effortlessly.
- **Rust-DSL Migrations:** Scaffolding databases now uses pure Rust closures (`make:migration`) instead of raw SQL, giving developers strong typing and compile-time validation for schema building.

---

## [0.1.1] - 2026-05-25 ✨

### Added
- **AI-Native Engineering & AI-Friendliness** added to core pillars in `README.md` and `README.pt.md`.
- **Master Plan Roadmap update:** Introduced the AI-Native Design Pillar at the top of the development roadmap (`ROADMAP.md` and `ROADMAP.pt.md`).
- **CLI Code Generator:** Added the first code generator subcommand `cargo rullst make:controller <Name>` in `cargo-rullst`.
  - Normalizes controller name inputs (e.g. `UsersController` -> `users_controller`).
  - Scaffolds REST endpoints (`index`, `show`) pre-configured with the JSX-like `html!` macro.
  - Automatically manages mod declarations in `src/controllers/mod.rs` and injects `pub mod controllers;` in `src/main.rs`.
- **CLI Path Normalization:** Normalized workspace and package names when scaffolding projects using path expressions (e.g. `cargo rullst new ..\my_project`).

---

## [0.1.0] - 2026-05-24 🚀

### Added
- **Core Crate (`rullst`):** Wrapped Axum server, routing macro `routes!`, lifecycle DB injection, and response models.
- **Macros Engine (`rullst-macros`):** Built procedural compiler-level `html!` JSX macro with static memory-string concatenation and dynamic XSS protection.
- **Developer CLI (`cargo-rullst`):** Scaffolds complete starter workspaces with integrated sqlite in-memory testing out-of-the-box.
- **Manifestos:** Created rich English (`README.md`) and Portuguese (`README.pt.md`) project overviews.
