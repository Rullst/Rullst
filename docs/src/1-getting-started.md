# Getting Started

Welcome to the **Rullst** Getting Started guide!

Rullst is a blazing-fast, strictly-typed Full-Stack web framework designed to give you a pristine developer experience while prioritizing security and zero-allocation performance.

## 1. Installation

First, ensure you have Rust installed. The official and recommended way is to visit [rustup.rs](https://rustup.rs/).

**For macOS and Linux:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**For Windows:**
Download and run `rustup-init.exe` from the website.

Next, install the **Rullst CLI**. The CLI is the heart of the developer experience, handling scaffolding, static sites (RullstPress), database migrations, and more.

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


## 3. Rullst Blueprints Showcase

The Rullst framework accelerates your development by providing **Blueprints**. A Blueprint is a highly-polished, pre-built application template that serves as the foundation for your project.

When you run `cargo rullst`, the wizard will ask you to select a Blueprint. All Blueprints feature the official Rullst color scheme (Emerald Green and Orange) and are built with zero-allocation HTML macros and HTMX.

## 1. Blank Starter
**Use Case:** Custom, from-scratch development.
This is the minimal template. It includes a simple HTMX reactive counter to demonstrate the frontend-backend communication, but leaves the rest entirely up to you.

## 2. Portfolio 🔥
**Use Case:** Developer showcases and personal branding.
**Status:** HOT!
A visually stunning, glassmorphic portfolio template designed specifically for Rullst/AI developers. It includes:
- A responsive Hero section with glowing text.
- Project cards with hover animations.
- A built-in contact form.

## 3. LMS Platform
**Use Case:** Online course platforms and video streaming.
A complete learning management system featuring:
- Courses and Lessons database models.
- Migrations pre-populated with seed data.
- A glassmorphic video player layout integrated with HTMX.

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

## 6. Uptime Monitor
**Use Case:** Infrastructure tracking and observability.
A robust system designed to ping URLs and track their health. It features:
- A dashboard with live status indicators.
- A background worker (`rullst::runtime::spawn`) that loops every minute to check endpoints.
- *Note: Background workers include a startup delay to ensure the database pool is fully initialized before querying.*

## 7. ERP Pocket
**Use Case:** Business management, stock, and inventory tracking.
A complete back-office suite out of the box. It features:
- A complex relational database schema (Products and Orders).
- Full CRUD operations with HTMX.
- A sleek, split-pane dashboard for simultaneous product listing and order creation.

---

> [!TIP]
> **Blueprint Evolution:** We are constantly refining our Blueprints. In version 2.0.1, all Blueprints were rigorously audited to ensure 100% macro syntax compliance (`routes!` and `html!`) and perfect database initialization order. You can build upon them with absolute confidence.

## 4. Modular Workspace & Optional Features

As of version **5.1.0**, Rullst features a highly-optimized **Modular Workspace Architecture**. To provide the fastest possible compilation times and the smallest binary sizes, Rullst's peripheral crates are now **100% optional**.

The main `rullst` crate acts as a facade. By default, it includes only the core HTTP and routing engines. Additional capabilities are activated via Cargo features:
- `auth`: Includes `rullst-auth` (Passkeys, OAuth, Argon2, Session Management).
- `ai`: Includes `rullst-ai` (Gemini, OpenAI, Anthropic, Ollama drivers).
- `capital`: Includes `rullst-capital` (Stripe and LemonSqueezy billing webhooks).
- `mailer`: Includes `rullst-mail` (SMTP integration).
- `nexus`: Includes `rullst-nexus` (Auto-generated CMS and Admin Panels).
- `studio`: Includes `rullst-studio` (Local Web-based Database Studio).

**How it works seamlessly:** 
You don't need to manually configure these! When you run `cargo rullst new` (or simply `cargo rullst`), the interactive scaffolding wizard intelligently analyzes your chosen Blueprint and automatically injects exactly the features you need into your `Cargo.toml`. 

For example, if you choose the **SaaS App Starter**, the CLI will automatically activate `auth`, `capital`, `mailer`, `nexus`, and `studio`. It will also ask if you want to include `ai` features! If you choose a simpler template like the **Blank Starter**, these heavy modules are completely skipped, drastically slashing your initial compile times.
