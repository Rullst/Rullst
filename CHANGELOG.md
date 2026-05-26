# Changelog 📝

All notable changes to the **Rullst Framework** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.3] - 2026-05-26 🛠️

### Fixed
- **CLI wizard prompt restoration**: Restores and guarantees the advanced CLI prompts (no-spaces validation, Full-Stack App vs Headless API selection, Hot reloading toggle, Database configuration toggle, and MySQL/MariaDB provider option) that were reverted in the 1.0.2 release due to a translation sync conflict.

## [1.0.2] - 2026-05-26 🚀

### Added
- **Rullst CLI Interactive Wizard (`cargo rullst new`) Improvements**:
  - Restricts application names to contain "no spaces allowed".
  - Adds descriptive options to select application type ("What would you like to build?": Full-Stack Web App vs Headless REST API).
  - Prompts to enable/disable Hot Reloading by default during scaffolding.
  - Prompts to configure database support ("Will your project need a Data Base?").
  - Adds "MySQL/MariaDB" provider selection option alongside SQLite and PostgreSQL.
- **RullstPress General-Purpose SSG**:
  - Capitalized CLI command descriptions and help menus to correctly read "RullstPress".
  - Updated documentation tutorial in `docs/2-tutorial-rullstpress.md` to introduce RullstPress as a general-purpose, high-performance, and multi-purpose Static Site Generator perfect for SaaS landing pages, wikis, blogs, and personal portfolios, rather than just documentation.

### Documentation
- Updated `README.pt.md` and `README.md` to reflect the new interactive CLI wizard questions and choices.

## [1.0.1] - 2026-05-26 🛡️

### Added
- **RullstPress (Native SSG)**:
  - `cargo rullst docs build`: Compiles all `.md` files in the `docs/` folder into static HTML files inside `docs/dist/`.
  - `cargo rullst docs dev`: Starts a live-preview local server for your documentation powered by Axum.
  - Automatically parses Markdown (via `pulldown-cmark`) and renders a premium dark-mode sidebar layout.

### Security & Quality Fixes
- **Security Enhancements**:
  - Implemented SHA-256 key derivation in `auth.rs` to securely stretch `APP_KEY` for AES-256-GCM.
  - Added safe `serde_urlencoded` parser to `security.rs` to guarantee CSRF tokens are safely extracted and compared from deeply nested url-encoded forms.
  - Restored strict HTML template string sanitization via template literals inside `error_console.rs` to prevent JS injection vectors.
- **Stability & Performance Fixes**:
  - Eliminated `.unwrap()` calls in `server.rs`, migrating `HotSwapService` to use graceful fallbacks that prevent runtime panics when dylibs are missing or file handles are locked.
  - Migrated dynamic library historical handles to `Mutex<Vec<Library>>` to safely retain historical pointers, preventing `libloading` Drop implementations from immediately freeing hot-swapped memory boundaries resulting in Segmentation Faults.
  - Refactored `scheduler.rs` loop to use `tokio::spawn` instead of blocking `await` on cron jobs, avoiding scheduler deadlock.
  - Migrated `queue.rs` SQLite worker to decouple popping from the database driver and loop latency, removing sleep-based latency blocks.
  - Fixed TOML parser bug in `mail.rs` resolving arbitrary `.unwrap()` when casting integer ports to unsigned integers.
  - Enabled inline comment stripping for `feature.rs` file reads to support `#` comments inside `Rullst.toml`.
  - Added background Cache Janitor to `cache.rs` via `tokio::spawn` using interval loops to actively prune expired DashMap keys.

## [1.0.0] - 2026-05-25 🚀


### Added (The "Unfair Advantage" & Local AI Dev Tooling)
- **Hot Reloading via Dynamic Linking (`Server::new_hot`)**:
  - Implemented `HotSwapService` wrapping `Arc<RwLock<axum::Router>>` for atomic in-flight router replacement without restarting the server or dropping TCP connections.
  - `Server::new_hot(lib_path)` builder that loads the application router from a `cdylib` (`.dll` / `.so`) at runtime via `libloading`.
  - Background file-watcher thread (using `notify`) that monitors `src/` for changes, debounces events (300ms), triggers `cargo build --lib`, and hot-swaps the router on success.
  - Timestamp-based unique DLL naming (`_active_{nanos}.dll`) to prevent Windows OS error 32 (file-locked-by-process), with automatic cleanup of stale copies.
  - FFI entry point convention: libraries export `#[unsafe(no_mangle)] pub extern "C" fn rullst_router_init() -> *mut rullst::Router`.
  - Blog example refactored to demonstrate hot-reload mode: `HOT_RELOAD=1 cargo run` for live-editing, default `cargo run` for standard static compilation.
- **Declarative E2E Testing (`rullst::testing`)**:
  - Introduced a fluent, high-level testing framework for complete application workflows.
  - Added `TestClient` to mount and run HTTP routing logic over the Axum application without actual TCP binding.
  - Implemented standard HTTP builders with convenient `.await` execution via Rust's `IntoFuture` trait.
  - Provided extensive cookie-based assertions (`.assert_cookie()`) and structured payload assertions (`.assert_json_value()`).
- **Built-in Feature Flags (`rullst::feature`)**:
  - Implemented full-stack toggles and dynamic A/B test splits with zero external runtime dependencies.
  - Support for `EnvDriver`, `MemoryDriver`, `TomlDriver`, and `DatabaseDriver` (backed by SQLx with a thread-safe TTL Cache for near-zero latency DB lookups).
  - High-performance deterministic consistency hash utilizing a custom MurmurHash3 implementation for stable weighted rollouts.
- **AI-Powered "Self-Healing" Error Console (`rullst::error_console`)**:
  - Gorgeous interactive glassmorphic web dashboard (`rullst-ignition`) triggered on application panics.
  - Seamless tokio panic interception using a custom `std::panic::set_hook` implementation to isolate runtime worker thread crashes.
  - Direct local code-snippet lookup pointing to the exact file, module, and line index where the panic occurred.
  - Integrated local AI-healing assistant that resolves runtime errors and can patch files directly back to the physical disk on a single web interface click.

### Security & Quality Audit Fixes (Audit 2026-05-25)
- **Security Enhancements**:
  - SEC-1: Removed unsafe `std::env::set_var("RUST_BACKTRACE", "1")` in `server.rs` (unsound in multi-threaded environments) and replaced it with a safe warning prompting the user to set the env var.
  - SEC-2: Added strict path traversal protection to the `/_rullst/autofix` endpoint in `error_console.rs` (verifies paths are canonicalized and located within the project root, restricts edits to `.rs` and `.toml` files).
  - SEC-3: Added a startup warning in `auth.rs` when the default development `APP_KEY` is used.
  - H-3 (Path Traversal in Error Console): Secured the GET `/_rullst/explain` handler in `error_console.rs` with robust path traversal validation, restricting file reads to `.rs` and `.toml` files within the workspace root.
  - H-1 (Poisoned RwLock Recovery): Added poison-recovery safety logic to `RwLock` reads/writes in `server.rs`, preventing a single dynamic loading thread panic from cascading to crash all request tasks.
  - H-2 (Graceful Oneshot Error Handling): Gracefully handle `oneshot()` failures inside tower routing, returning an internal server error response instead of panicking.
- **Spec & API Alignments & Stability**:
  - Marked `Server`, `Router`, `HtmxRequest`, and `HtmxResponse` as `#[non_exhaustive]` per Rullst Spec §9.1 to ensure future-proof API stability.
  - Replaced a `panic!` in `Storage::disk()` with a graceful fallback `ErrorDriver` returning `StorageError::DriverError` on all methods when an unknown disk is requested.
  - M-2 (Stable Rollout Hashing): Replaced `DefaultHasher` in progressive rollouts (`feature.rs`) with deterministic `FnvHasher` (adding `fnv` to main dependencies) to guarantee bucket stability across Rust upgrades.
  - L-3 (TOML Path Isolation): Cached `Rullst.toml`'s path during construction in `TomlFeatureDriver` to prevent lookup failure if the runtime working directory changes.
  - L-4 (Removed Undocumented Tenancy Fallback): Removed the undocumented `"tenant"` parameter fallback in `multitenant.rs` to enforce explicit, predictable tenancy extraction.
- **Performance & Reliability**:
  - Migrated `LocalDriver` in `storage.rs` from blocking `std::fs` to fully asynchronous `tokio::fs` operations.
  - Optimized Redis `CacheDriver`'s `flush()` method to use a memory-efficient `SCAN` cursor loop instead of the blocking `KEYS *` pattern.
  - M-1 (Watcher Compilation Timeout): Implemented a `120s` timeout for background `cargo build --lib` compilation using std channel `recv_timeout` to prevent blocking the watcher indefinitely.
  - M-4 (Configurable Testing Limits): Made the E2E testing request body limit configurable in `TestApp` and `TestRequestBuilder`, and provided comprehensive panic error details if limits are exceeded.
  - L-1 (Guaranteed Temp DLL Uniqueness): Swapped timestamp suffixes with UUID v4 to completely rule out dynamic library path collision bugs under high concurrent loads.
- **UX & Diagnostics Improvements**:
  - I-2 (Hot-Reload Panic Capture Console): Wrapped `HotSwapService`'s execution future in a spawned task, intercepting panic unwinds to render the gorgeous glowing interactive Self-Healing Console during development.
  - L-2 (HTML Attribute Injection Guard): Implemented robust HTML attribute escaping to `ws_path` before mounting Live component tags inside `live.rs`.
- **Testing & CI/CD**:
  - Added full test coverage for the wrapper `Router` in `routing.rs`, the builder in `server.rs`, and argument translation in `artisan.rs`.
  - Created a GitHub Actions CI pipeline (`.github/workflows/ci.yml`) enforcing automated test suites, clippy lint checks, and rustfmt checks.


## [0.8.0] - 2026-05-25 🛡️

### Added (Self-Healing Upgrades & Architectures)
- **Architectural Guidelines (`docs/spec.md`)**:
  - Enforced the Builder Pattern and `#[non_exhaustive]` on public configurations to prevent struct instantiation breakages.
  - Formally integrated `#[deprecated]` lifecycle for smooth transition between APIs.
  - Implemented the "Sealed Traits" pattern for internal interfaces.
- **Automated CLI Upgrade Command (`cargo-rullst`)**:
  - Added `cargo rullst upgrade` command.
  - Safely updates dependencies via `cargo update -p rullst`.
  - Automatically runs codemods using `cargo fix --allow-no-vcs --allow-dirty` to apply Rust compiler suggestions based on Rullst's deprecation warnings.

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
