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

Next, install the **Rullst CLI**. The CLI is the heart of the developer experience, handling scaffolding, database migrations, dev server, and more.

```bash
cargo install cargo-rullst
```

## 2. Creating Your First Project

We have completely redesigned the project creation experience. Instead of remembering complex flags, just run:

```bash
cargo rullst
```

The **Rullst App Creator** will launch an interactive wizard. Let's create a beautiful Portfolio:
1. Select **Create New App**.
2. **App Name**: Provide a simple lowercase name (e.g., `my_portfolio`).
3. **Starter Blueprint**: Choose **Portfolio 🔥 (showcase for Rullst/AI developers) - HOT**.

```bash
cd my_portfolio
cargo rullst dev
```

> [!TIP]
> The `cargo rullst dev` command automatically compiles your code and spins up a local server. If you edit any `.rs` file, it will instantly recompile using Hot Reload!
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
The ultimate boilerplate for SaaS products, pre-wired with:
- User authentication (login, signup, session management).
- Stripe pricing panels and subscription checkout views.
- Secure user dashboard.

## 5. Blog / Press
**Use Case:** Content creation and articles.
A static site generator pre-wired with Nexus CMS. It features:
- A beautiful article reading view with typography optimized for readability.
- A fully functional Markdown parser engine.
- SEO-friendly metadata injection.

## 6. ERP Pocket
**Use Case:** Business management, stock, and inventory tracking.
A complete back-office suite out of the box. It features:
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

This convenience cannot be enabled in a release binary through `APP_ENV`. A
release build does not start the generated Studio task and requires unique
`NEXUS_ADMIN_USERNAME` and `NEXUS_ADMIN_PASSWORD` values before Nexus can be
constructed. Put production Nexus behind a verified TLS boundary and explicit
application authorization; do not expose Studio publicly.

## 5. Database Configuration & Turso Cloud Integration

Rullst ORM supports **SQLite**, **Turso / libSQL**, **PostgreSQL**, and **MySQL**.

### Local-First Experience (Zero Config)
When creating a project with **SQLite** or **Turso / libSQL**, Rullst defaults to
a local file database (`db.sqlite` or `turso_local.db`). This reduces initial
setup, but filesystem permissions, migrations, and external Turso connectivity
still need to be configured and tested.

```env
# Default local DATABASE_URL generated by cargo rullst
DATABASE_URL=sqlite://turso_local.db?mode=rwc
```

### Switching to Turso Cloud
Turso is powered by **libSQL**, an open-source extension of SQLite built for edge databases. When you are ready to deploy to Turso Cloud:

1. Create a database via Turso CLI or dashboard:
   ```bash
   turso db create my-app-db
   turso db tokens create my-app-db
   ```
2. Update your `.env` file with your Turso Cloud credentials:
   ```env
   DATABASE_URL=libsql://my-app-db-username.turso.io?authToken=eyJhbGciOi...
   ```

### Running Turso Server Locally (`turso dev`)
If you prefer running a local Turso/libSQL server instance instead of file-based SQLite:
```bash
turso dev --db-file dev.db
```
Then update your `.env`:
```bash
DATABASE_URL=http://127.0.0.1:8080
```
