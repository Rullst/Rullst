# 💡 Rullst CLI - Full Command Reference

The Command Line Interface (`cargo-rullst`) is the heart of the Rullst ecosystem. It doesn't just create files; it acts as a static analyzer, infrastructure orchestrator, and Wasm compiler.

Below is the **exhaustively detailed** reference for absolutely all commands and their flags.

---

## 🏗️ 1. Project Initialization & Maintenance

### `cargo rullst new <name>`
Creates a Rullst project from scratch. This command presents an interactive wizard prompting for project options:
* **Starter Blueprint:** Blank Starter, Portfolio, LMS Platform, SaaS App, Blog/Press, ERP Pocket.
* **ORM Architecture:** Active Record (`User::find(id)`), Data Mapper / Repository (`UserRepository::find()`), or Hybrid.
* **Frontend Engine:** Zero-Bundle HTMX + TailwindCSS (0KB JS default), LiveView Server-Driven UI (`rullst::live`), Reactive Wasm Islands (`rullst::island`), Zero-Build Semantic CSS (Pico.css v2), or File-Based Classic Templates (Tera).
* **Arguments:**
  * `<name>`: The folder and package name (e.g., `my_startup`).
* **Optional Flags:**
  * `--api`: Scaffolds a headless JSON API project (no HTML view rendering).
  * `--docker`: Adds multi-stage `Dockerfile`, `docker-compose.yml`, and `.dockerignore`.
  * `--turso`: Scaffolds Turso/libSQL sidecar (`sqld`) configuration for edge database replication.
  * `--nix`: Adds `flake.nix` and `.envrc` (direnv) files for a 100% reproducible environment.

### `cargo rullst upgrade`
Globally updates the CLI on your machine (`cargo install cargo-rullst`) and simultaneously scans your current project's `Cargo.toml` to ensure Rullst and its internal macros (like `rullst-macros` and `rullst-connect`) are updated to the corresponding stable version, running automatic codemods if breaking changes are detected.

### `cargo rullst pkg <action> [name]`
Manages third-party community packages and extensions conforming to the `RullstPackage` trait standard.
* **Subcommands:**
  * `add <package_name>`: Injects a community extension dependency (e.g., `cargo rullst pkg add rullst-auth`) into `Cargo.toml`.
  * `list`: Scans and lists all active `rullst-*` community extensions installed in your project.

---

## 🛠️ 2. Architecture Scaffolding (`make:*`)

Rullst is heavily opinionated. All `make:*` commands automatically update references (e.g., registering a controller in the main router) and regenerate the **AI Context** (`.llms.txt`).

### `cargo rullst make:resource <name>`
Scaffolds a complete Full CRUD resource stack in a single command. It simultaneously generates the Model (`src/models/<name>.rs`), Migration (`migrations/<timestamp>_create_<name>s_table.rs`), Controller (`src/controllers/<name>.rs`), and HTML Views (`views/<name>/index.html` & `views/<name>/form.html`).
* **Arguments:** `<name>` (e.g., `Product` or `product`).
* **Optional Flags:**
  * `--api`: Scaffolds a headless JSON API resource controller instead of HTML views.

### `cargo rullst make:controller <name>`
Generates a new Controller in the `src/controllers/` directory. It creates the standard CRUD methods (`index`, `show`, `create`, `update`, `delete`) and automatically registers the route in `main.rs`.
* **Arguments:** `<name>` (e.g., `UsersController` or `users`).
* **Optional Flags:**
  * `--api`: Instead of returning HTML Views via the `html!` macro, the generated methods will automatically extract/return `Json<T>`.

### `cargo rullst make:model <name>`
Creates a Model Struct in the `src/models/` directory with the ORM annotations (`#[derive(Model)]`).
* **Arguments:** `<name>` (e.g., `BlogPost`).
* **Optional Flags:**
  * `--migration` or `-m`: Simultaneously generates an empty SQL migration file (`src/migrations/YYYYMMDD_create_blog_posts.sql`) with the correctly pluralized table name.

### `cargo rullst make:middleware <name>`
Generates a standard Axum/Rullst Middleware struct in `src/middlewares/`. Perfect for injecting headers, checking authentication, rate limiting, or logging.

### `cargo rullst make:island <name>`
Creates a frontend interactive "Islands Architecture" component (similar to Fresh or Astro) in `src/islands/`. It generates the Rust infrastructure that, during build, will be transparently compiled to WebAssembly to run in the browser.

### `cargo rullst make:worker <name>`
Creates an asynchronous background Job (Worker) in `src/workers/`. If you pass `Email`, it generates an `EmailWorker` that consumes queues in the background (Redis/RabbitMQ).

### `cargo rullst make:migration <name>`
Generates a raw SQL migration file (Up and Down) prefixed with a timestamp, guaranteeing the correct chronological execution order in the database.

### `cargo rullst make:billing`
Scaffolds a complete SaaS system. It generates `Subscription` models, webhook integrations, billing dashboard routes, and prepares the shell for providers like Stripe or LemonSqueezy.

### `cargo rullst make:jwt`
Injects a pre-configured boilerplate Middleware into your project for strict JWT Authentication (verifying Bearer tokens in the `Authorization` header).

### `cargo rullst make:cors`
Generates and configures full CORS (Cross-Origin Resource Sharing) options in your project with recommended security defaults (blocking unused methods, restricting origins).

### `cargo rullst make:omni`
Prepares your project to become a Desktop or Mobile App. It generates Tauri/Omni manifests, creating the native bridge so you can package your website as an `.exe` or `.apk`.

### `cargo rullst make:iot <DeviceName>`
Scaffolds an IoT edge device module (Sensor Node, MQTT Gateway) in `src/iot/` pre-configured with `rullst-iot` telemetry models and MQTT/CoAP protocol formatters.

### `cargo rullst make:k8s`
Scaffolds cloud-native Kubernetes manifest files in the `k8s/` directory (`deployment.yaml`, `service.yaml`, `configmap.yaml`, `hpa.yaml`, `ingress.yaml`, and `all-in-one.yaml`) pre-configured with liveness (`/health`) and readiness (`/ready`) HTTP probes.

### `cargo rullst make:scalar`
Scaffolds an interactive Scalar API Documentation controller at `src/controllers/docs_controller.rs` serving modern OpenAPI UI at `http://localhost:3000/docs`.

### `cargo rullst make:live <ComponentName>`
Scaffolds a new LiveView-style reactive server component at `src/live/<name>.rs` enabling real-time WebSocket state synchronization and HTMX Out-Of-Band (OOB) HTML swaps without writing JavaScript.

### `cargo rullst make:grpc <ServiceName>`
Scaffolds a new gRPC service implementation in `src/grpc/<name>.rs` and Protobuf schema definition in `proto/<name>.proto` powered by `tonic`.

### `cargo rullst deploy [--platform <fly|railway|render|vps>]`
1-Click deployment wizard generating cloud manifests (`fly.toml`, `railway.json`, `render.yaml`, `docker-compose.prod.yml` with Caddy SSL) and launching the deployment.

### `cargo rullst auth`
The Supreme Command. With just one command, it creates an entire Authentication system in your codebase, including:
- User Model and Migration (with `bcrypt`/`argon2` password hashing).
- Auth Controllers (Login, Registration, Logout).
- Session or Token Middleware.
- Complete HTML Views for Login and Signup (unless `--api` is used).

### `cargo rullst make:mfa`
Scaffolds a 2FA TOTP Multi-Factor Authentication controller at `src/controllers/mfa.rs` providing RFC 6238 Base32 secret generation, 6-digit TOTP code validation, and `otpauth://` QR URI generation.

---

## 🗄️ 3. Database and Migrations (`db:*`)

### `cargo rullst db:migrate`
Analyzes the internal `_rullst_migrations` table in your database and executes all SQL files in the `migrations/` directory that haven't been run yet.

### `cargo rullst db:rollback`
Reverts the last applied migration batch. It looks at the latest executed batch, extracts the "Down" section of the SQL file, and executes it to undo changes and remove tables/columns.

### `cargo rullst db:status`
Checks the database connection and prints a table in the terminal comparing the local `migrations/` folder with the database status, detailing exactly what has been run and what is pending.

### `cargo rullst db:seed`
Populates the database using seeder files created in `src/db/seeds.rs`, ideal for injecting an initial administrator or dummy testing data.

### `cargo rullst studio`
Launches an internal web server on port `:5555` that functions as the **Rullst Visual Database Studio**. It is a visual administration panel where you can edit records, run queries, and visualize table relationships directly in the browser.

---

## 🧠 4. Analyzers and Code Generators (`generate:*`)

### `cargo rullst generate:openapi`
Reads your source code via Regular Expressions and AST without needing to compile the project. It extracts routes, decipher URL parameters, extracts `///` Rustdoc comments from controllers, and generates a fully compliant OpenAPI V3 `openapi.json` file.

### `cargo rullst generate:ts`
Scans your models, DTOs (Data Transfer Objects), and mapped routes. Transcribes all Rust structs into a strictly typed TypeScript file (`sdk.ts`), eliminating contract breaks between frontend and backend.

### `cargo rullst generate:diagram`
Analyzes primary and foreign keys defined in your Models and exports a `diagram.md` file containing Mermaid.js code, visually generating an Entity-Relationship (ER) diagram.

### `cargo rullst generate:models` / `cargo rullst make:models-from-db`
Connects to a legacy database (that already exists and has tables), maps the entire "Information Schema", and automatically outputs Rust Struct files based on the columns and types found in the database.
* **Required Flags:**
  * `--driver`: `postgres`, `mysql`, or `sqlite`.
  * `--url`: The complete connection string.
* **Optional Flags:**
  * `--output`: Where to save the generated structs (Default: `src/models`).

### `cargo rullst generate:ai-context`
Creates the brain map of your project (`.llms.txt`). It summarizes the folder structure, conventions, and dependencies so that AI Assistants (like Cursor and Github Copilot) perfectly understand the framework when you ask them for help.

### `cargo rullst audit [--ai] [--compliance] [--idor]`
Runs an AI-assisted security audit scanning `.env` for secret leaks, verifying dependency CVEs via `cargo audit`, detecting IDOR/BOLA authorization gaps in parameterized routes, and evaluating RBAC permission boundaries with AI Sentinel suggestions.
* **Flags:**
  * `--ai`: Enables AI Sentinel suggestions for threat mitigation.
  * `--compliance`: Generates a `SECURITY_COMPLIANCE.md` report evaluating OWASP Top 10, SOC2 Type II, and ISO 27001 control requirements.
  * `--idor`: Runs recursive static AST analysis across all routes (`/:id`, `/{id}`, `/users/:user_id`) to verify if ownership checks (`RbacGuard::authorize_owner_or_role`) are properly enforced.

### `cargo rullst eject [--force] [--output <path>]`
Expands all Rullst framework abstractions into 100% pure Axum and Tokio Rust code (`src/ejected_main.rs`), eliminating framework lock-in and allowing low-level Tower/Hyper customization.
* **Flags:**
  * `--force`: Overwrites `src/main.rs` directly instead of creating `src/ejected_main.rs`.
  * `--output <path>`: Specifies a custom output path for the ejected file.

### `cargo rullst inspect [target]`
Statically expands and inspects macro code or structural definitions directly in the terminal without starting a server. Useful for debugging proc-macro output, reviewing route tables, and validating database schemas.
* **Arguments:**
  * `[target]`: The item or file to inspect:
    * `route` or `routes`: Renders the active route table (methods, paths, and handlers).
    * `model` or `models`: Renders ORM struct models and field attributes.
    * `schema`: Outputs the project's structural JSON schema (`rullst-schema.json`).
    * `<path/to/file.rs>`: Displays the first 40 lines of any target Rust file with line numbers.

---

## 🚀 5. Development, Infrastructure, and Build

### `cargo rullst dash`
Opens the Interactive Dashboard (TUI - Terminal User Interface) powered by Ratatui. It splits your screen in half, displaying colorful server logs, the build system events, stats, and allows running migrations at the touch of a key (hotkeys).

### `cargo rullst dev`
Runs the classic Rullst development server in the terminal with Hot-Reload. Any modification to `.rs` or HTML files will instantly restart the server at lightning speed.
* **Optional Flags:**
  * `--ts-sync`: Automatically watches controller and model file changes and syncs the TypeScript client SDK (`sdk.ts`) live during development.

### `cargo rullst build:client`
Extracts all "Islands" (client-side interactivity) from your code and compiles them via `wasm-pack` into tiny WebAssembly binaries, ready to run at the speed of light in the browser.
* **Flags:** `--debug` (Avoids extreme minification so you can inspect and debug Wasm sourcemaps).

### `cargo rullst build`
Creates the monolithic final Production binary of the backend and executes pre-compression tools (GZIP and Brotli) on your static assets.
* **Flags:** `--debug` (Compiles with debug information, generating a larger binary).

### `cargo rullst dockerize` / `cargo rullst nixify`
Injects infrastructure files (Dockerfile or Nix Flake) directly into a pre-existing project (similar to the flags used in `new`).

### `cargo rullst foundry:init`
Generates the `Foundry.toml` deployment manifest at the project root containing SSH access settings (host IP, user, SSH key, deploy path) and environment variables for direct Bare-Metal / Cloud VPS deployment (Hetzner, DigitalOcean, AWS EC2, Linode, Vultr). Automatically adds `Foundry.toml` to `.gitignore`.

### `cargo rullst foundry:deploy`
Executes an automated 5-step SSH deployment pipeline against your remote Linux server: local release build (`cargo build --release`), remote directory & systemd provisioning via SSH, `scp` binary transfer with SHA-256 integrity verification, remote database migrations (`cargo rullst db:migrate`), and zero-downtime service reload with HTTP health probes (`GET /health`).

### `cargo rullst omni`
Initializes your native application client (after using `make:omni`), launching the operating system window.
* **Optional Arguments:** `<target>` specifies where to run (e.g., `desktop`, `android`, `ios`).

---

## 🛠️ Quick CLI Cheat Sheet

```bash
# Create a new project with fast-linker scaffolding
cargo rullst new my_app

# Reverse-engineer ORM models from an existing database
cargo rullst make:models-from-db --driver postgres --url "postgres://user:pass@localhost:5432/mydb"

# Statically inspect routes, models, or schemas in the terminal
cargo rullst inspect route
cargo rullst inspect model

# Launch the visual Studio Dashboard (Data Browser, ER Diagram, Feature Flags)
cargo rullst studio

# Deploy to Cloud / VPS with automatic SSL Caddy setup
cargo rullst foundry:deploy
```
