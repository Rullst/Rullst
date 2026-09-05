# 💡 Rullst CLI - Full Command Reference

The Command Line Interface (`cargo-rullst`) scaffolds projects, invokes build
tools, and provides bounded static-analysis and deployment helpers.

The CLI's `--help` output is authoritative for the installed version. This page
documents the principal version 12 commands and their security boundaries.

---

## 🏗️ 1. Project Initialization & Maintenance

### `cargo rullst new <name>`
Creates a Rullst project from scratch. This command presents an interactive wizard prompting for project options:
* **Starter Blueprint:** Blank Starter, Portfolio, LMS Platform, SaaS App, Blog/Press, ERP Pocket.
* **ORM Architecture:** Active Record (`User::find(id)`), Data Mapper / Repository (`UserRepository::find()`), or Hybrid.
* **Persistence:** a primary relational backend (SQLite, PostgreSQL, MySQL,
  MariaDB, or bounded Turso-primary for blank/API) plus optional Turso/libSQL,
  MongoDB, DuckDB, SurrealDB, and Qdrant capabilities. The optional selector
  accepts zero or more choices and omits capabilities already selected by the
  primary profile or flags. Specialized adapters remain separate from SQLx
  Active Record.
* **Frontend profile:** HTMX + Tailwind SSR is the audited default. The LiveView,
  Wasm Island, Pico.css and Tera selections record compatibility intent and add
  limited dependencies/scaffold markers; they do not yet generate four complete,
  interchangeable application renderers. Wire and test the selected runtime and
  browser assets explicitly.
* **Arguments:**
  * `<name>`: The folder and package name (e.g., `my_startup`).
* **Optional Flags:**
  * `--api`: Scaffolds a headless JSON API project (no HTML view rendering).
  * `--docker`: Adds the current multi-stage `Dockerfile` packaging scaffold; Compose services and deployment hardening remain explicit project work.
  * `--turso`: Adds the direct Hrana HTTP v3 Turso/libSQL adapter, checked migrations, and its real-SQL offline development fallback to the selected primary backend. It does not imply transparent replication.
  * `--mongodb`: Enables typed MongoDB document CRUD and its deterministic offline store.
  * `--duckdb`: Enables in-process DuckDB analytics; the optional native dependency increases the first build time.
  * `--surrealdb`: Enables SurrealDB HTTP document CRUD and bounded read-only graph queries.
  * `--qdrant`: Enables bounded dense-vector Qdrant operations and generates empty/`mock_*`-compatible environment fields; it is additive, not the SQL primary.
  * `--nix`: Adds `flake.nix` and `.envrc` (direnv) starting points; reproducibility still depends on pinned inputs and external services.
  * `--buildah`: Adds rootless Buildah container-build files where supported.
  * `--default`: Uses deterministic non-interactive defaults, intended for CI and reproducible scaffolding.
  * `--blueprint <blank|lms|saas|blog|portfolio|erp>`: Selects a blueprint when used with `--default`.
  * `--database <sqlite|postgres|mysql|mariadb|turso>`: Selects the primary relational backend with `--default`; network databases must be configured before migration bootstrap. Turso-primary currently supports the blank/API starter and rejects SQLx-specific blueprints explicitly.
  * `--no-database`: Generates the blank blueprint without a primary relational database; it conflicts with `--database` and rejects database-dependent blueprints and explicit ORM choices.
  * `--orm <active-record|repository|hybrid>`: Selects the generated ORM architecture with `--default`. Turso-primary selects its bounded typed Active Record profile and rejects an incompatible override.
  * `--frontend <htmx|liveview|wasm-island|pico|tera>`: Selects the recorded frontend profile with `--default`, subject to the frontend limitations above.
  * `--hot-reload`: Generates the library boundary used by the development hot-reload workflow. Unsupported Turso-primary and detached LMS combinations fail explicitly.
  * `--ai`: Enables the umbrella AI facade in the generated manifest.
  * `--redis`: Enables the umbrella Redis queue/cache/ORM capabilities and the direct ORM Redis feature.
  * `--lms-modules <modules>`: With `--default --blueprint lms`, selects a detached LMS profile. Version 12 currently accepts `auth`, `auth,learning`, or `auth,learning,assessment`; unsupported/duplicate combinations and the profiles' not-yet-supported hot reload fail explicitly. Omitting the flag generates the complete LMS starter.
  * `--skip-initial-migration`: Generates the project without running the best-effort initial database migration. Run `cargo rullst db:migrate` explicitly after configuring the database.

Without `--skip-initial-migration`, project creation performs the first Cargo
build before applying migrations. A clean first build can take several minutes,
especially for the larger LMS/SaaS profiles; the animated status remains visible
while Cargo is working. Later migration and server runs reuse that project-local
build cache.

For example, the release gate can generate a SaaS starter without prompts or
network-dependent bootstrap work:

```bash
cargo rullst new packaged-saas --default --blueprint saas --skip-initial-migration
```

A complete deterministic profile can pin every supported generation axis:

```bash
cargo rullst new operations-portal --default --blueprint erp \
  --database mariadb --orm hybrid --frontend liveview \
  --hot-reload --ai --redis --skip-initial-migration
```

Generated SQLx applications disable the umbrella dependency's default features
and select exactly one strict primary profile (`strict-sqlite`,
`strict-postgres`, or `strict-mysql`; MariaDB uses the MySQL protocol). This
prevents an implicit SQLite default from masking the chosen backend.

#### Generated-project verification boundary

The repository does not treat template rendering as sufficient evidence. A
structural contract materializes all 270 supported blueprint/profile shapes and
checks paths, Rust syntax and manifests. A slower eight-case set crosses every
blueprint, hot and non-hot layouts, database/ORM/frontend/API boundaries and a
release build, runs every generated test target, and constructs the public
router of each hot-reload project using offline-safe defaults. A separate
seven-case test invokes the public `cargo rullst new` binary and verifies exact
feature selection for SQLite, PostgreSQL, MySQL, MariaDB, AI, Redis, Turso,
MongoDB, DuckDB, SurrealDB and Qdrant across all six public blueprints plus a
polyglot profile. The invocation starts outside the source checkout, proving
that an unpublished pre-release CLI retains its exact matching checkout as a
path source instead of requesting unavailable registry packages.

The public polyglot profile uses `cargo check` in that CLI-level set because a
second bundled-DuckDB test build adds no adapter behavior and can consume
several GiB on small machines. DuckDB, MongoDB, SurrealDB, Turso and Qdrant
runtime behavior is exercised by their dedicated ORM matrices instead. These
gates prove reproducible local generation and bounded offline construction;
they do not prove provider accounts, production deployment, browser behavior
or application-specific authorization.

The bounded LMS foundation omits assessment, gamification, automation and
notification files while retaining authenticated catalog/enrollment/progress:

```bash
cargo rullst new academy-identity --default --blueprint lms \
  --lms-modules auth --skip-initial-migration

cargo rullst new academy-foundation --default --blueprint lms \
  --lms-modules auth,learning --skip-initial-migration

cargo rullst new academy-assessment --default --blueprint lms \
  --lms-modules auth,learning,assessment --skip-initial-migration
```

The assessment foundation adds owner-only quiz presentation and
server-authoritative, idempotent grading with bounded attempts. It deliberately
does not pull in scoring, leaderboards, achievements, automation, outbox, or
notification modules.

### `cargo rullst upgrade`
Plans or applies a transactional application upgrade. The target defaults to
the exact installed `cargo-rullst` version; `--to <VERSION>` accepts an exact
version in the same major release train as that CLI.

```bash
# Human-readable plan; no writes or dependency resolution
cargo rullst upgrade --dry-run

# Versioned machine-readable plan
cargo rullst upgrade --dry-run --json

# Backed-up apply + cargo fix + cargo check
cargo rullst upgrade

# Deliberately inspect a failed partial migration instead of auto-rollback
cargo rullst upgrade --keep-on-failure

# Recover a persisted snapshot, including after interruption
cargo rullst upgrade --restore target/rullst-upgrades/<run-id>
```

The CLI uses Cargo metadata to scope workspace manifests, preserves TOML
comments/order, updates normal, inline, workspace, target-specific and renamed
Rullst dependencies, and reports unversioned path/git entries. Before applying,
it snapshots workspace manifests, the root `Cargo.lock`, and Rust sources under
`target/rullst-upgrades/`; a failed Cargo gate restores them by default. The
reports use the `rullst.upgrade-plan.v1` schema and include version-selected
source findings.

Process-level fixtures select the rule catalog independently for documented v5,
v6 and v11 origins, verify atomic restoration across multiple workspace
members, retain a deliberately failed edit only with `--keep-on-failure`, and
restore that retained snapshot on demand. Symlinked Rust sources are rejected
before a transaction begins. This is recovery evidence for the bounded file and
Cargo operation; it is not an automatic application, database or deployment
migration.

The command does not install the CLI globally, rewrite Axum/SQLx/Tokio imports,
run database migrations, modify secrets or authorization, validate live
providers, or replace the project's test suite. Follow the
[assisted upgrade tutorial](tutorials/36-assisted-framework-upgrades.md) and the
relevant [v12 migration guide](migration-v12.md).

### `cargo rullst pkg <action> [name]`
Manages third-party community packages and extensions conforming to the `RullstPackage` trait standard.
* **Subcommands:**
  * `add <package_name>`: Injects a community extension dependency (e.g., `cargo rullst pkg add rullst-auth`) into `Cargo.toml`.
  * `list`: Scans and lists all active `rullst-*` community extensions installed in your project.

---

## 🛠️ 2. Architecture Scaffolding (`make:*`)

Rullst generators write the files described under each command. Some commands
also register modules and refresh `.llms.txt`; this is command-specific, and a
failed best-effort context refresh does not roll back generated source. Review
the diff and run `cargo check` after scaffolding.

### `cargo rullst make:resource <name>`
Scaffolds the bounded starting files for a CRUD resource in one command: a
Model (`src/models/<name>.rs`), Migration
(`migrations/<timestamp>_create_<name>s_table.rs`), Controller
(`src/controllers/<name>.rs`), and HTML view placeholders
(`views/<name>/index.html` and `views/<name>/form.html`). It does not infer
application fields, register routes, establish ownership/RBAC, or turn the
placeholder handlers into a complete authorized CRUD implementation. Mount the
routes behind the canonical security baseline, render request-scoped CSRF
tokens in state-changing forms, complete validation/persistence, and run the
application's authorization-negative tests.
* **Arguments:** `<name>` (e.g., `Product` or `product`).
* **Optional Flags:**
  * `--api`: Scaffolds a headless JSON API resource controller instead of HTML views.

### `cargo rullst make:controller <name>`
Generates a new Controller in the `src/controllers/` directory. It creates
placeholder CRUD methods (`index`, `show`, `store`, `update`, `delete`) and
registers the Rust module in `main.rs` when that file exists; it does not add
application routes automatically.
* **Arguments:** `<name>` (e.g., `UsersController` or `users`).
* **Optional Flags:**
  * `--api`: Instead of returning HTML Views via the `html!` macro, the generated methods will automatically extract/return `Json<T>`.

### `cargo rullst make:model <name>`
Creates a model struct in `src/models/` with the ORM annotations. SQLx projects
receive `FromRow` plus `Orm`; Turso-primary projects receive
`#[derive(rullst_orm::Orm)] #[orm(backend = "turso")]` and an `i64` primary
key. Backend detection reads the generated manifest and does not treat an
additive `--turso` integration as the primary ORM.
* **Arguments:** `<name>` (e.g., `BlogPost`).
* **Optional Flags:**
  * `--migration` or `-m`: Simultaneously generates a reversible migration with the correctly pluralized table name.

### `cargo rullst make:chat-session`

Adds application-owned conversational memory for the project's primary ORM.
It generates and registers `ChatSession` and `ChatMessage`, a reversible
migration, and `StatefulChat`. SQLx and the bounded Turso-primary profile receive
backend-specific code; the command also enables the `orm` and `ai` umbrella
features if necessary.

```bash
cargo rullst make:chat-session
cargo rullst db:migrate
```

Save the generated `ChatSession` before constructing `StatefulChat`. Each
service instance serializes concurrent sends, restores at most the newest 100
messages in chronological order, persists the user message before provider
dispatch and persists the assistant response only after success. Database and
provider failures are returned as `StatefulChatError`; they are never silently
discarded. Multi-process ordering, tenant authorization, retention and deletion
remain application responsibilities. The command refuses to overwrite an
existing chat scaffold.

### `cargo rullst make:middleware <name>`
Generates a standard Axum/Rullst Middleware struct in `src/middlewares/`. Perfect for injecting headers, checking authentication, rate limiting, or logging.

### `cargo rullst make:island <name>`
Creates a frontend interactive "Islands Architecture" component (similar to Fresh or Astro) in `src/islands/`. It generates the Rust infrastructure that, during build, will be transparently compiled to WebAssembly to run in the browser.

### `cargo rullst make:worker <name>`
Creates an asynchronous background worker in `src/workers/` against the queue
backends currently implemented in Core (memory, SQLite, and optional Redis).
RabbitMQ is not generated by this command.

### `cargo rullst make:migration <name>`
Generates a timestamped reversible Rust migration for the project's primary
backend. SQLx projects use the schema DSL; Turso-primary projects use
`TursoMigration` and parameterized `TursoStatement` values, and regenerate a
fallible typed migration registry.

### `cargo rullst make:billing`
Scaffolds a SaaS billing starting point with subscription models, authenticated
billing routes, and signed-webhook integration points. Provider credentials,
tenant policy, and deployment behavior still require application configuration.

### `cargo rullst make:mail <Name>`
Scaffolds a registered transactional mailable. `--welcome`, `--reset`, `--otp`
and `--invoice` select the bounded built-in variants; without a flag the command
generates a custom message type. It enables the umbrella `mailer` feature, uses
the `rullst::mail` facade, escapes dynamic HTML and refuses invalid identifiers,
path traversal or an existing target. Delivery credentials, URL semantics,
tenant policy and provider operation remain application responsibilities.

### `cargo rullst make:mail-invoice [Name]`

Generates `FiscalInvoiceEmail` by default and enables `mailer` plus `capital`.
The result supports an international commercial receipt and an NFS-e message
constructed from typed `FiscalResponse` provenance. An `OfflineMock` is always
rendered as `[PREVIEW — NOT AUTHORIZED]`; the generator cannot turn local DPS,
XSD, or XMLDSig validity into a tax authorization. A custom valid struct name
may be supplied positionally.

### `cargo rullst make:mail-dunning [Name]`

Generates `PaymentDunningEmail` by default with explicit gentle D+1,
action-required D+3, and service-status D+7 stages. The application remains
responsible for calculating the due state, scheduling delivery, enforcing its
disclosed billing policy, and reconciling payment. The generated build path
runs the mandatory pre-flight and rejects dangerous links.

### `cargo rullst make:jwt`
Injects a pre-configured boilerplate Middleware into your project for strict JWT Authentication (verifying Bearer tokens in the `Authorization` header).

### `cargo rullst make:cors`
Generates and configures full CORS (Cross-Origin Resource Sharing) options in your project with recommended security defaults (blocking unused methods, restricting origins).

Projects generated by older CLI versions retain the middleware that was copied
into their source tree and must be reviewed manually. Follow the
[CORS scaffold security advisory](cors-scaffold-security-advisory.md) to detect
origin reflection/wildcards and migrate to the current fail-closed allowlist.

### `cargo rullst make:omni`
Generates a Tauri/Omni shell and development configuration for desktop, Android
or iOS. Interactive use prompts for platforms. Automation can select one or
more targets deterministically:

```bash
cargo rullst make:omni --platform desktop
cargo rullst make:omni --platform android \
  --backend-url http://10.0.2.2:3000 --identifier com.acme.myapp
cargo rullst make:omni --platform ios \
  --backend-url https://app.example.com --identifier com.acme.myapp
cargo rullst make:omni --platform desktop,ios \
  --backend-url https://app.example.com --identifier com.acme.myapp \
  --product-name "Acme App" --app-version 1.2.3
```

Mobile generation requires an explicit backend URL. HTTPS is required except
for the bounded localhost/Android-emulator development hosts; embedded
credentials are rejected. Mobile also requires an application-owned lowercase
reverse-DNS `--identifier`; reserved framework and `com.example` placeholders
are rejected. `--product-name` and `--app-version` are optional validated
overrides and otherwise inherit the host package metadata. Desktop-only
development can derive a documented `com.example` placeholder, which must be
replaced before distribution.

The generator installs an exact Tauri npm CLI, creates platform icons,
initializes mobile targets non-interactively, emits a restrictive local CSP and
fails if a requested prerequisite step fails. iOS initialization requires
macOS and Xcode. Native-side navigation is restricted to the packaged
bootstrap and the configured backend's exact origin. Remote pages receive no
privileged Tauri IPC surface; cross-origin OAuth/external-link behavior needs a
separate reviewed system-browser/deep-link integration.

The canonical product remains the Rullst web application and the generated
client packages that application; it does not by itself
implement native plugins, offline synchronization, production network policy,
release signing, privacy declarations, physical-device validation, Play
Store/App Store publication or review acceptance. The generated README contains
the application-owned distribution checklist. Path-aware repository workflows
generate fresh desktop, Android and iOS shells and compile only their declared
targets; those runs are packaging evidence, not store, physical-device or
universal behavior guarantees.

### `cargo rullst make:iot <DeviceName>`
Scaffolds and registers a telemetry-only IoT module in `src/iot/` using the
public `rullst::iot::SensorTelemetry` facade, and enables the `iot` feature in
the application manifest. Unsafe identifiers/path traversal and existing target
files are rejected. It does not install an MQTT/CoAP transport, HAL, firmware,
or claim broker connectivity.

### `cargo rullst make:k8s`
Scaffolds cloud-native Kubernetes manifest files in the `k8s/` directory (`deployment.yaml`, `service.yaml`, `configmap.yaml`, `hpa.yaml`, `ingress.yaml`, and `all-in-one.yaml`) pre-configured with liveness (`/health`) and readiness (`/ready`) HTTP probes.

### `cargo rullst make:scalar`
Scaffolds a Scalar API Documentation controller at
`src/controllers/docs_controller.rs`. The interactive view loads a pinned CDN
asset; its local fallback is status-only and final CSP/network policy belongs to
the application.

### `cargo rullst make:live <ComponentName>`
Scaffolds a LiveView-style server component at `src/live/<name>.rs` using a
WebSocket and HTMX out-of-band swaps. Application JavaScript may be unnecessary,
but HTMX remains client-side JavaScript and the generated transport requires
origin, reconnect, and backpressure review.

### `cargo rullst make:grpc <ServiceName>`
Scaffolds a new gRPC service implementation in `src/grpc/<name>.rs` and Protobuf schema definition in `proto/<name>.proto` powered by `tonic`.

### `cargo rullst deploy [--platform <fly|railway|render|vps>]`
Guided deployment helper that generates cloud manifests (`fly.toml`,
`railway.json`, `render.yaml`, or `docker-compose.prod.yml`) and invokes the
selected provider CLI where supported. Credentials, migrations, availability,
DNS/TLS and rollback remain operator responsibilities.

### `cargo rullst auth`
Creates an authentication starting point in your codebase, including:
- User model and migration with asynchronous Argon2 password hashing.
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
Launches the local developer Studio on port `:5555`. Treat it as a privileged
development tool; do not expose it publicly without an independently reviewed
authentication, authorization, and TLS boundary.

---

## 🧠 4. Analyzers and Code Generators (`generate:*`)

### `cargo rullst generate:openapi`
Reads recognizable route and Rustdoc patterns and generates an OpenAPI V3 draft.
Dynamic routes, custom extractors, and semantic constraints may require manual
edits; validate the result with an OpenAPI validator before publishing it.

### `cargo rullst generate:ts`
Scans supported models and DTOs and emits a TypeScript file (`sdk.ts`). Generated
types reduce duplication but do not replace compatibility tests for serialization
and API behavior.

### `cargo rullst generate:diagram`
Analyzes primary and foreign keys defined in your Models and exports a `diagram.md` file containing Mermaid.js code, visually generating an Entity-Relationship (ER) diagram.

### `cargo rullst generate:models` / `cargo rullst make:models-from-db`
Connects to an existing database and generates reviewable starter structs from
the tables and columns visible in SQLite or the current PostgreSQL/MySQL schema.
Table lookups are parameterized and SQL identifiers are allowlisted. Table
module names are normalized, while collisions and database columns that would
require an unsupported ORM field remapping fail before the output directory is
written. The bounded type mapping falls back to `String`; review keys,
relations, custom types, schema selection and generated files before compiling
or replacing application models.
* **Required Flags:**
  * `--driver`: `postgres`, `mysql`, or `sqlite`.
  * `--url`: The complete connection string.
* **Optional Flags:**
  * `--output`: Where to save the generated structs (Default: `src/models`).

### `cargo rullst generate:ai-context`
Creates `.llms.txt`, a compact summary of project structure, conventions, and
dependencies for coding assistants. It is context, not a guarantee that a model
will understand or modify the project correctly.

### `cargo rullst audit [--ai] [--compliance] [--idor]`
Runs bounded source/configuration checks and can invoke installed dependency
scanners. Static findings require human review and are not a penetration test or
compliance certification.
* **Flags:**
  * `--ai`: Enables AI Sentinel suggestions for threat mitigation.
  * `--compliance`: Generates an evidence-oriented control report with `PASS`, `FAIL`, `SKIPPED`, or `NOT_EVALUATED`; it does not confer SOC 2 or ISO 27001 certification.
  * `--idor`: Fails on parameterized routes without an adjacent `// rullst-access: public|owner|role|admin — reason` classification and the recognized guard required by non-public classifications. `public` is accepted only for recognized GET routes. This bounded heuristic cannot prove domain authorization correctness.

### `cargo rullst eject [--force] [--output <path>]`
Generates an inspectable Axum/Tokio entry-point snapshot
(`src/ejected_main.rs`) for the supported abstractions. Review it and run
`cargo check`; optional subsystems may still depend on Rullst crates.
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
Opens the Ratatui development control surface in an interactive terminal. The
dashboard reports the probed application port, explicit hot-reload profile
state, the child process exit state, and the configured database profile; it
does not label a database as connected merely because a URL exists. Logs and
input queues are bounded, ANSI control sequences are removed, terminal state is
restored on error, and the owned application process is stopped and reaped when
the dashboard exits.

The layout adapts to narrower terminals and provides these keyboard controls:

* `o`: open the application.
* `s`: probe the loopback Studio endpoint and open it only when reachable.
* `d`: open existing Scalar docs. Missing files produce explicit
  `cargo rullst make:scalar` guidance rather than silently modifying the project.
* `m`: run `db:migrate` asynchronously and report its real exit result.
* `/`: search both log panes; `f` cycles all/warning+error/error filtering.
* `Tab`: switch the focused log pane; arrows and Page Up/Page Down scroll it.
* `c`: clear dashboard logs; `q` or `Esc`: exit.

The animated neon palette is enabled only for an interactive terminal. Set
`RULLST_REDUCED_MOTION=1` to keep colors with static rendering, or `NO_COLOR=1`
for a color-free, static interface. Non-interactive automation should use
`cargo rullst dev`; `dash` fails clearly when no terminal is attached.

### `cargo rullst dev`
Runs the development server. Projects generated with the explicit
`--hot-reload` profile receive the bounded dynamic-library workflow below;
other projects run normally and the CLI explains that source swapping is not
enabled instead of pretending to watch them.

For a hot-reload profile, the CLI:

1. detects and coalesces changed Rust/manifest paths before starting a build;
2. runs `cargo build --lib` asynchronously and captures a bounded diagnostic;
3. keeps the currently serving router unchanged when compilation fails;
4. sends a five-second, session-token-authenticated loopback reload request only
   after a successful build;
5. serializes router swaps, lets in-flight requests finish on their prior
   router, and reports the measured build-plus-swap duration;
6. tells connected browsers to perform a reliable full-page refresh through a
   same-origin, offline JavaScript client and same-origin WebSocket.

The token is a fresh 256-bit value passed to the owned application process; it
is not printed. Direct unauthenticated calls to the internal reload route fail
closed. To avoid unloading code that an in-flight request might still execute,
the development server retains loaded library generations and stops accepting
swaps at 64 total libraries. Restart `cargo rullst dev` or `cargo rullst dash`
at that point to release them.

This is a debug/development facility built on a Rust-ABI-compatible `cdylib`,
not a production plugin ABI. It does not migrate arbitrary process state or
preserve unsaved browser state, and a view-only AST classification still uses a
real Rust compilation. Rullst therefore makes no universal “sub-millisecond”
claim: the CLI prints the observed duration, which depends on the application,
cache, linker, toolchain, and machine.

See [Authenticated Development Hot Reload](tutorials/51-authenticated-hot-reload.md)
for the complete lifecycle, failure table, security boundary, and troubleshooting
expectations.

* **Optional Flags:**
  * `--ts-sync`: Automatically watches controller and model file changes and syncs the TypeScript client SDK (`sdk.ts`) live during development.

### `cargo rullst build:client`
Builds the library for `wasm32-unknown-unknown`, runs `wasm-bindgen`, and writes a
separate `static/rullst-islands.js` hydrator that awaits binding initialization.
It parses `Cargo.toml`, merges the required `cdylib` crate type without replacing
existing library crate types, and honors an explicit `lib.name`. The command
checks/installs the Rust target and `wasm-bindgen-cli`; any failed tool step
aborts. Bundle size and browser performance depend on the generated application
and must be measured.
* **Flags:** `--debug` (Avoids extreme minification so you can inspect and debug Wasm sourcemaps).

### `cargo rullst build`
Creates the monolithic final Production binary of the backend and executes pre-compression tools (GZIP and Brotli) on your static assets.
* **Flags:** `--debug` (Compiles with debug information, generating a larger binary).

### `cargo rullst dockerize` / `cargo rullst nixify`
Injects infrastructure files (Dockerfile or Nix Flake) directly into a pre-existing project (similar to the flags used in `new`).

### `cargo rullst foundry:init`
Generates the `Foundry.toml` deployment manifest at the project root containing
SSH access settings and environment variables for a compatible systemd-based
Linux VPS. It adds `Foundry.toml` to `.gitignore`; operators must still verify
that secrets were never committed.

### `cargo rullst foundry:deploy`
Executes an SSH deployment pipeline: local release build, remote directory and
systemd provisioning, `scp` transfer, environment/Caddy configuration, service
restart, and a bounded remote-local `/health` probe. It requires a preinstalled,
reviewed `curl`, systemd, and Caddy installation plus root or passwordless
non-interactive `sudo`. Candidate files are staged under an application-specific
`/opt/rullst/<app>` root, the Caddy configuration is validated, and `.previous`
copies of replaced files are retained. The current command replaces the global
`/etc/caddy/Caddyfile`; it does not perform a separate remote checksum,
migrations, data backup, external reachability check, or automatic rollback. It
does not guarantee zero downtime and does not support IPv6 SCP targets.

### `cargo rullst omni`
Runs the generated Tauri development client after `make:omni`. Android/iOS
require their official SDK/toolchain and a reachable backend.
* **Optional Arguments:** `<target>` specifies where to run (e.g., `desktop`, `android`, `ios`).

---

## 🛡️ 4. Security, Compliance & System Diagnostics

### `cargo rullst audit`
Executes bounded automated checks across recognized source, configuration,
route, dependency, and local network patterns.
* **Optional Flags:**
  * `--ai`: Enables autonomous AI Sentinel analysis with risk assessment and proactive remediation advice.
  * `--compliance`: Generates an evidence-oriented control report; it is not a SOC 2, ISO 27001, or transport certification.
  * `--idor`: Fails on parameterized routes without an explicit adjacent access classification. `owner` requires `RbacGuard::authorize_owner_or_role`; `role` requires a recognized role guard; `admin` requires `RequireRoleLayer` or `NexusAuthPolicy::protect_router`; `public` is restricted to recognized GET routes. Manual review and runtime negative tests remain required.
  * `--geiger`: Inventories `unsafe` in the dependency tree. Unsafe may be justified and requires review; the command does not prove a zero-unsafe invariant.
  * `--sbom`: Generates a standardized **CycloneDX 1.5 JSON** Software Bill of Materials (`sbom-cyclonedx.json`) with package SHA-256 checksums and license metadata.
  * `--audit-ignore RUSTSEC-YYYY-NNNN`: Passes one explicit, repeatable advisory exception to `cargo audit`. A successful run is reported as **NO FINDINGS OUTSIDE EXCEPTIONS**, not “no findings”; the caller must separately version, own, review, and expire every exception.
  * `--network`: Checks a bounded list of local ports/bindings for potentially exposed services; it is not a comprehensive network scan.

### `cargo rullst hook:install`
Installs managed `pre-commit` and `commit-msg` wrappers. The first runs
`cargo fmt --all -- --check`, strict workspace Clippy, and
`cargo rullst audit --idor`; the second enforces Conventional Commits. Existing
active hooks are moved to explicit `.rullst-original` backups and invoked first,
while reinstalling the managed wrappers is idempotent. The command supports
linked worktrees, fails clearly outside a Git worktree, and refuses a backup
collision instead of overwriting it. These local hooks are bypassable by design;
protected CI remains authoritative.

### `cargo rullst doctor`
Runs bounded system and toolchain diagnostics for Rust MSRV (>= 1.96.0),
linters, `cargo-llvm-cov`, `cargo-audit`, `cargo-geiger`, `cargo-deny`,
`cargo-mutants`, `kani-verifier`, and Docker Engine, and reports detected or
missing components.

### `cargo rullst inspect [target]`
Expands macros and displays structural insights in the terminal:
* `cargo rullst inspect route`: Lists all registered HTTP, WebSocket, and gRPC endpoints.
* `cargo rullst inspect model`: Inspects ORM model columns, primary keys, and relationships.
* `cargo rullst inspect schema`: Displays the synchronized database schema.

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

# Run the reviewed Foundry pipeline on a compatible, prepared VPS
cargo rullst foundry:deploy
```
