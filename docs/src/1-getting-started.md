# Getting Started

Welcome to the **Rullst** Getting Started guide!

Rullst is a strictly typed full-stack web framework designed around explicit APIs,
measurable performance, and defense-in-depth defaults.

## 1. Installation

First, ensure you have Rust installed. The official and recommended way is to visit [rustup.rs](https://rustup.rs/).

**For macOS and Linux:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**For Windows:**
Download and run `rustup-init.exe` from the website.

Next, install the **Rullst CLI** from the same release train as the framework.
The registry command installs the latest published release; it does not install
the unreleased v12 source documented by this branch:

```bash
cargo install cargo-rullst
```

To evaluate v12 before its first RC is published, clone this repository and
install the CLI from that exact checkout instead:

```bash
git clone --branch main https://github.com/Rullst/Rullst.git
cd Rullst
cargo install --locked --path cargo-rullst
```

Run the following project-creation steps from the repository root during this
source-only phase. The generator will then use the sibling framework crates as
path dependencies. Once an immutable v12 RC exists on crates.io, install that
exact CLI version and use its matching registry packages instead.

## 2. Creating Your First Project

We have completely redesigned the project creation experience. Instead of remembering complex flags, just run:

```bash
cargo rullst
```

The **Rullst App Creator** will launch an interactive wizard. The example below
creates a Portfolio inside the source checkout while v12 remains unpublished:
1. Select **Create New App**.
2. **App Name**: Provide a simple lowercase name (e.g., `my_portfolio`).
3. **Starter Blueprint**: Choose **Portfolio 🔥 (showcase for Rullst/AI developers) - HOT**.

```bash
cd my_portfolio
cargo rullst dev
```

> [!TIP]
> The `cargo rullst dev` command compiles the project and starts the local
> server. When the project was generated with `--hot-reload`, a change triggers
> a real library rebuild, an authenticated router swap, and a browser refresh;
> a failed build leaves the previous router serving. The CLI reports observed
> duration rather than promising a fixed reload time. See the bounded behavior
> and limitations in the [CLI reference](cli_reference.md#cargo-rullst-dev).
> 
> For a detailed guide on choosing ORM patterns (Active Record vs Data Mapper vs Hybrid) and Frontend Engines (HTMX vs Leptos vs Dioxus), check out our [Architecture Choices Guide](architecture-decisions.md).


## 3. Rullst Blueprints Showcase

The Rullst framework accelerates your development by providing **Blueprints**. A Blueprint is a highly-polished, pre-built application template that serves as the foundation for your project.

When you run `cargo rullst`, the wizard asks you to select a blueprint. The
blueprints use the Rullst color scheme and server-rendered HTML/HTMX patterns;
allocation and latency depend on the generated page and runtime.

## 1. Blank Starter
**Use Case:** Custom, from-scratch development.
This is the minimal template powered by HTMX without a project-local JavaScript
bundle. It includes a simple reactive counter to demonstrate server-driven
communication. You can select one of five frontend scaffolds (HTMX, LiveView,
Wasm Islands, Pico.css, or Tera) or one of the blueprints below.

## 2. Portfolio 🔥
**Use Case:** Developer showcases and personal branding.
**Status:** HOT!
A visually stunning, glassmorphic portfolio template designed specifically for Rullst/AI developers. It includes:
- **Profile Settings in Nexus CMS (`/nexus`)**: Edit your name, title, bio, email, website URL, avatar photo, GitHub, and LinkedIn links live without changing code.
- A responsive sidebar and Hero section with glowing glassmorphism effects.
- Interactive Experience timeline and Skills tags.
- Project cards showcase with live external links.

## 3. LMS Platform Starter
**Use Case:** Online course platforms and video streaming.
An initial learning-management scaffold featuring:
- Courses and Lessons database models.
- Migrations pre-populated with seed data.
- A glassmorphic video player layout integrated with HTMX.

Enrollment/entitlements, student progress, assessments, certificates, protected
video delivery, uploads/transcoding, notifications and native offline playback
are not implemented by this starter. They are worthwhile application modules,
but require domain authorization, durable jobs/storage and provider contracts.

## 4. SaaS App Starter
**Use Case:** Subscription-based products and billing.
An opinionated SaaS starting point, pre-wired with:
- User authentication (login, signup, session management).
- Stripe pricing panels and subscription checkout views.
- Secure user dashboard.

## 5. Blog / Press
**Use Case:** Content creation and articles.
A static site generator pre-wired with Nexus CMS. It features:
- A beautiful article reading view with typography optimized for readability.
- Article CRUD and a server-rendered reading view. Markdown parsing is not part
  of the current generated starter.
- SEO-friendly metadata injection.

## 6. ERP Pocket
**Use Case:** Business management, stock, and inventory tracking.
An inventory-oriented back-office starter. It features:
- A complex relational database schema (Products and Orders).
- Full CRUD operations with HTMX.
- A sleek, split-pane dashboard for simultaneous product listing and order creation.

---

> [!TIP]
> **Blueprint evolution:** Blueprints are continuously checked by generator
> smoke tests. Treat generated code as an application starting point: inspect its
> configuration and rerun `cargo check` and security tests after customization.

### Local Studio and Nexus access

The Blog, Portfolio, LMS, ERP, and SaaS blueprints expose visible buttons for
both control surfaces during local development:

- **Nexus** is mounted at `/nexus` and accepts only a verified loopback peer in a
  debug build, so the first local click needs no placeholder password.
- **Studio** starts as a separate debug-only service at
  `http://127.0.0.1:5555`.

This convenience cannot be enabled in a release binary through `RULLST_ENV` or
the legacy `APP_ENV` alias. A
release build does not start the generated Studio task and requires unique
`NEXUS_ADMIN_USERNAME` and `NEXUS_ADMIN_PASSWORD` values before Nexus can be
constructed. Put production Nexus behind a verified TLS boundary and explicit
application authorization; do not expose Studio publicly.

## 5. Database configuration and specialized stores

The wizard deliberately makes two separate decisions:

1. Choose the primary **SQLx Active Record** backend: **SQLite**,
   **PostgreSQL**, **MySQL**, or **MariaDB**. MariaDB uses the MySQL wire
   protocol but has its own executable container contract. The blank/API
   starter can instead select **Turso/libSQL** as its primary typed ORM.
2. Optionally add **Turso/libSQL** edge SQL to a SQLx application, **MongoDB**
   documents, **DuckDB** analytics, or **SurrealDB** documents/graph reads.
   These use explicit capability APIs and do not silently replace the primary
   pool.

The non-interactive flags mirror the second step:

```bash
cargo rullst new edge_app --default --database mariadb --turso --mongodb \
  --skip-initial-migration
```

Automation can pin the ORM, frontend and optional runtime capabilities instead
of accepting hidden interactive defaults:

```bash
cargo rullst new learning_portal --default --blueprint lms \
  --database postgres --orm repository --frontend htmx --ai \
  --skip-initial-migration
```

For a blank application with no primary relational database, use the explicit
`--no-database` flag. It cannot be combined with `--database` or `--orm`.
Generated SQLx profiles disable Rullst's umbrella defaults and select exactly
one strict relational backend, so a chosen PostgreSQL/MySQL/MariaDB profile is
not accidentally compiled through an implicit SQLite default.

To create a Turso-primary API using the current bounded blank starter:

```bash
cargo rullst new edge_app --default --api --database turso \
  --skip-initial-migration
cd edge_app
cargo rullst db:migrate
```

The generated `.env` selects a persistent, real-SQL offline fallback and does
not invent a SQLx `DATABASE_URL`:

```env
TURSO_DATABASE_URL=mock_local
TURSO_AUTH_TOKEN=
TURSO_OFFLINE_PATH=turso-development.db
```

To use Turso Cloud, put the token in its own variable rather than in the URL:

```bash
turso db create my-app-db
turso db tokens create my-app-db
```

```env
TURSO_DATABASE_URL=libsql://my-app-db-username.turso.io
TURSO_AUTH_TOKEN=replace-with-a-secret-token
```

The familiar derive explicitly selects the Turso backend:

```rust
#[derive(Debug, Clone, rullst_orm::Orm)]
#[orm(table = "users", backend = "turso")]
struct User {
    id: i64,
    name: String,
}
```

`User::all()`, `find`, `save`, `create`, `delete`, `count`, filtering,
ordering and pagination then execute through the primary `TursoOrm` store.
`make:model --migration`, `make:migration`, `db:migrate`, `db:status`, and
`db:rollback` preserve this backend. The current first-class scaffold contract
is deliberately limited to the blank/API profile; SQLx-specific LMS, SaaS,
blog, portfolio, and ERP blueprints reject `--database turso` until ported.
Transparent embedded-replica synchronization is not claimed.

`TursoStore` remains available for prepared SQL, bounded result
materialization, transactions and checksummed migrations in either primary or
additive configurations.
See [Polyglot Persistence](polyglot-persistence.md) for complete setup,
security constraints and examples for every optional store.
