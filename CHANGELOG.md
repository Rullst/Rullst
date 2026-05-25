# Changelog 📝

All notable changes to the **Rullst Framework** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
