# 💡 Rullst CLI - Full Command Reference

The Command Line Interface (`cargo-rullst`) is the heart of the Rullst ecosystem. It doesn't just create files; it acts as a static analyzer, infrastructure orchestrator, and Wasm compiler.

Below is the **exhaustively detailed** reference for absolutely all commands and their flags.

---

## 🏗️ 1. Project Initialization & Maintenance

### `cargo rullst new <name>`
Creates a Rullst project from scratch. This command doesn't just clone a template; it builds the directory tree, injects the database connection into `.env`, creates the `Cargo.toml` with the correct features, and generates a `main.rs` prepared with routing configurations.
* **Arguments:**
  * `<name>`: The folder and package name (e.g., `my_startup`).
* **Optional Flags:**
  * `--api`: Changes the default scaffolding behavior. Instead of preparing a Full-Stack project with HTML rendering, it generates a headless project (JSON API only), focusing strictly on JSON and removing template dependencies.
  * `--docker`: Automatically adds a highly optimized multi-stage `Dockerfile` for Rust, a `docker-compose.yml` (with the database), and the `.dockerignore`.
  * `--nix`: Adds `flake.nix` and `.envrc` (direnv) files to create a 100% reproducible development environment isolated from the host OS.

### `cargo rullst upgrade`
Globally updates the CLI on your machine (`cargo install cargo-rullst`) and simultaneously scans your current project's `Cargo.toml` to ensure Rullst and its internal macros (like `rullst-macros` and `rullst-connect`) are updated to the corresponding stable version, running automatic codemods if breaking changes are detected.

---

## 🛠️ 2. Architecture Scaffolding (`make:*`)

Rullst is heavily opinionated. All `make:*` commands automatically update references (e.g., registering a controller in the main router) and regenerate the **AI Context** (`.llms.txt`).

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

### `cargo rullst auth`
The Supreme Command. With just one command, it creates an entire Authentication system in your codebase, including:
- User Model and Migration (with `bcrypt`/`argon2` password hashing).
- Auth Controllers (Login, Registration, Logout).
- Session or Token Middleware.
- Complete HTML Views for Login and Signup (unless `--api` is used).

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

### `cargo rullst generate:models`
Connects to a legacy database (that already exists and has tables), maps the entire "Information Schema", and automatically outputs Rust Struct files based on the columns and types found in the database.
* **Required Flags:**
  * `--driver`: `postgres`, `mysql`, or `sqlite`.
  * `--url`: The complete connection string.
* **Optional Flags:**
  * `--output`: Where to save the generated structs (Default: `src/models`).

### `cargo rullst generate:ai-context`
Creates the brain map of your project (`.llms.txt`). It summarizes the folder structure, conventions, and dependencies so that AI Assistants (like Cursor and Github Copilot) perfectly understand the framework when you ask them for help.

---

## 🚀 5. Development, Infrastructure, and Build

### `cargo rullst dash`
Opens the Interactive Dashboard (TUI - Terminal User Interface) powered by Ratatui. It splits your screen in half, displaying colorful server logs, the build system events, stats, and allows running migrations at the touch of a key (hotkeys).

### `cargo rullst dev`
Runs the classic Rullst development server in the terminal with Hot-Reload. Any modification to `.rs` or HTML files will instantly restart the server at lightning speed.

### `cargo rullst build:client`
Extracts all "Islands" (client-side interactivity) from your code and compiles them via `wasm-pack` into tiny WebAssembly binaries, ready to run at the speed of light in the browser.
* **Flags:** `--debug` (Avoids extreme minification so you can inspect and debug Wasm sourcemaps).

### `cargo rullst build`
Creates the monolithic final Production binary of the backend and executes pre-compression tools (GZIP and Brotli) on your static assets.
* **Flags:** `--debug` (Compiles with debug information, generating a larger binary).

### `cargo rullst dockerize` / `cargo rullst nixify`
Injects infrastructure files (Dockerfile or Nix Flake) directly into a pre-existing project (similar to the flags used in `new`).

### `cargo rullst foundry:init`
Prepares the `Foundry.toml` manifest, which maps environment variables and Cloud provider configurations (AWS, DigitalOcean) for 1-click deployment.

### `cargo rullst foundry:deploy`
Reads your `Foundry.toml` and, using the configured credentials, packages and orchestrates the deployment via API directly to your cloud, returning the final public URL of your application.

### `cargo rullst omni`
Initializes your native application client (after using `make:omni`), launching the operating system window.
* **Optional Arguments:** `<target>` specifies where to run (e.g., `desktop`, `android`, `ios`).
