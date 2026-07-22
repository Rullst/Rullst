<!-- “Antes de gerar qualquer coisa, leia e siga estritamente o arquivo da verdade abaixo. spec.md”  -->

# Rullst Specification 📄
### *"The Single Source of Truth (SST) for Framework Architecture & Conventions"*

This document is the **Single Source of Truth (SST)** for the **Rullst Framework**. It specifies the exact conventions, API structures, naming rules, and directory standards of Rullst.

> [!IMPORTANT]
> **AI Alignment Instruction:**
> Whenever updating, refactoring, or generating documentation and code for Rullst, **always** refer to this specification as the baseline. Do not invent or assume conventions outside of this document.

---

## 📂 1. Directory Structure Conventions

A standard Rullst application scaffold must strictly follow this folder hierarchy:

```text
my-app/
├── src/
│   ├── controllers/      # Route controllers (async modules)
│   │   └── mod.rs
│   ├── models/           # Active Record Models (rullst-orm entities)
│   │   └── mod.rs
│   ├── pages/            # Shared static HTML elements or full page layouts
│   │   └── mod.rs
│   └── main.rs           # Entrypoint, DB initialization, and Central routing
├── Cargo.toml            # Project cargo dependencies
└── Rullst.toml           # Framework configuration (databases, environment, etc.)
```

---

## 🛠️ 2. Naming Conventions

To guarantee consistency, both humans and AI coders must adhere to the following name normalization rules handled by the `cargo-rullst` generator:

* **File Names:** Standard Rust `snake_case` (e.g. `users_controller.rs`, `post_model.rs`).
* **Struct / Model / Documentation Names:** Standard `PascalCase` (e.g. `UsersController`, `PostModel`).
* **URL Paths:** Lowercase kebab-case (e.g. `/users`, `/user-profiles`).

---

## ⚡ 3. Core API Specifications

### 3.1. Server & Routing (`rullst::routing`)

* **Routing Macro:** central routing declared via the `routes!` macro, wrapping Axum routing handlers.
  ```rust
  let router = routes![
      get("/" => home),
      post("/posts" => posts_controller::store),
  ];
  ```
* **Server Lifecycle:**
  ```rust
  Server::new(router: Router)
      .run(port: u16) -> Result<(), Box<dyn std::error::Error>>
  ```

### 3.2. Server-Side Rendering (`rullst::macros`)

* **Macro:** `html!` procedural macro compiles HTML trees directly into static memory string concat builders.
* **XSS Protection:** Automatic HTML escaping on all dynamic variables wrapped in `{expr}`.
* **Raw Unescaped HTML:** Explicitly bypassed using the wrapper `rullst::html::RawHtml(String)`.
* **Lists/Iterators:**
  ```rust
  let mut list_builder = String::new();
  for item in items {
      list_builder.push_str(&html! { <li>{item}</li> });
  }
  html! { <ul>{ rullst::html::RawHtml(list_builder) }</ul> }
  ```

### 3.3. Active Record ORM (`rullst-orm`)

* **Model definition:**
  ```rust
  #[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
  #[orm(table = "table_name")]
  pub struct Model {
      pub id: i32,
      // ... fields
  }
  ```
* **Static queries:**
  * `Model::all().await` -> `Result<Vec<Model>, sqlx::Error>`
  * `Model::find(id).await` -> `Result<Model, sqlx::Error>`
* **Instance Operations:**
  * `let mut instance = Model { ... };`
  * `instance.save().await` -> `Result<(), sqlx::Error>` (handles auto-incrementing inserts or updates).
  * `instance.delete().await` -> `Result<(), sqlx::Error>`

---

## 💻 4. CLI Specifications (`cargo-rullst`)

* **Project Creation:**
  `cargo rullst new <name>`
  * *Convention:* Automatically extracts the package name from path expressions (e.g., `..\dummy_test` -> `dummy_test`).
* **Controller Scaffolding:**
  `cargo rullst make:controller <Name>`
  * *Behavior:* Generates `src/controllers/<snake_name>_controller.rs` with `index` and `show` actions. Appends declaration to `src/controllers/mod.rs`. Adds `pub mod controllers;` to the top of `src/main.rs`.
* **Documentation SSG (RullstPress):**
  `cargo rullst docs build` and `cargo rullst docs dev`
  * *Behavior:* Compiles markdown files in `docs/` into a static site inside `docs/dist/`.

---

## 🧱 5. Controller Architecture

Controllers handle business logic and HTTP responses.
* **Module Structure**: Each controller is a separate module inside `src/controllers/` (e.g., `users_controller.rs`).
* **Function Signatures**: Functions must be asynchronous and return a type that implements `axum::response::IntoResponse` (or `Result<impl IntoResponse, AppError>`).
* **Database Access**: Controllers must **never** contain raw `sqlx::query!` macros inline. Database logic must be delegated to the Active Record ORM methods (`.save()`, `.all()`, etc.) or encapsulated within specific `impl Model` functions.
* **Standard Actions:** 
  * `pub async fn index()`: List all resources.
  * `pub async fn show(Path(id): Path<i32>)`: Show a specific resource.
  * `pub async fn store(Form(payload): Form<CreateDto>)`: Create a new resource.
  * `pub async fn update(Path(id): Path<i32>, Form(payload): Form<UpdateDto>)`: Update a resource.
  * `pub async fn delete(Path(id): Path<i32>)`: Delete a resource.

---

## 📄 6. HTML Pages & Components

Rullst uses a functional approach for HTML rendering, relying on the `html!` macro.
* **Organization:** Pages and components reside in `src/pages/`.
* **Functional Components:** Pages and components are simply Rust functions. They are not structs or classes.
* **Props/Data:** Pass data into pages and components as regular function arguments.
* **Return Type:** Components should return a `String` (or `rullst::html::RawHtml`) so they can be embedded in other `html!` calls. Route-level pages should return `axum::response::Html<String>` to be served directly.
* **Example:**
  ```rust
  pub fn button_component(label: &str, url: &str) -> String {
      html! { <a href={url} class="btn">{label}</a> }
  }
  
  pub fn home_page(user_name: &str) -> axum::response::Html<String> {
      let content = html! {
          <div>
              <h1>"Welcome, "{user_name}</h1>
              { rullst::html::RawHtml(button_component("Click Me", "/click")) }
          </div>
      };
      axum::response::Html(content)
  }
  ```

---

## 🚨 7. Error Handling

Consistent error handling ensures safety and predictable API responses.
* **Default Error Type:** The framework expects a standard error enum, typically `AppError`, located in `src/error.rs` or similar.
* **Implementation:** `AppError` must implement `axum::response::IntoResponse`.
* **Controller Usage:** Controllers that can fail should return `Result<impl IntoResponse, AppError>`.
* **HTTP Codes:** The `IntoResponse` implementation maps internal errors to appropriate HTTP status codes (e.g., `404 Not Found`, `500 Internal Server Error`).

---

## 🛡️ 8. Middlewares

Middlewares intercept requests for authentication, logging, etc.
* **Location:** Middlewares are placed in `src/middlewares/`.
* **Standard Signature:** Following Axum's `from_fn` pattern, a middleware function looks like:
  ```rust
  use axum::{extract::Request, middleware::Next, response::Response};
  
  pub async fn my_middleware(req: Request, next: Next) -> Response {
      // Pre-request logic here
      let response = next.run(req).await;
      // Post-request logic here
      response
  }
  ```
* **Registration:** Middlewares are registered on the router using Axum's `.layer()` or through Rullst's server configuration wrapper.

---

## 🛡️ 9. Architectural Guidelines for Backward Compatibility

To guarantee stress-free and self-healing updates for Rullst users, all framework code must strictly adhere to the following backward compatibility rules:

### 9.1. The Builder Pattern and `#[non_exhaustive]`
Any public configuration struct or extensible enum exposed by the framework **must** use the `#[non_exhaustive]` attribute. This prevents developers from instantiating the struct directly, ensuring that adding new fields in future minor versions will not break user code.

* **Mandatory Usage:** All instantiation must be done via a constructor (`new()`) and the Builder Pattern (`with_...()`).

```rust
#[non_exhaustive]
pub struct RullstConfig {
    pub port: u16,
}

impl RullstConfig {
    pub fn new(port: u16) -> Self {
        Self { port }
    }
}
```

### 9.2. Deprecation Lifecycle (`#[deprecated]`)
The framework will never abruptly remove or rename a public function, struct, or method. If a breaking change to an API is required, the old API must be kept alive for at least one minor version using the `#[deprecated]` attribute.
* **Mandatory Usage:** The `note` field must explicitly tell the user what to use instead, enabling `cargo fix` to potentially automate the migration.

```rust
#[deprecated(since = "0.2.0", note = "Please use `Router::new()` instead")]
pub fn old_initializer() {
    Router::new();
}
```

### 9.3. Sealed Traits
If the framework exposes a Trait that is meant to be used by the user but **not implemented** by the user (e.g., core framework behavior), it must use the "Sealed Trait" pattern. This ensures that adding new methods to the trait in the future will not break downstream implementations.

```rust
mod private {
    pub trait Sealed {}
}

pub trait RullstTrait: private::Sealed {
    fn execute(&self);
}
```

---

## 🏗️ 10. CLI Modular Architecture (`cargo-rullst`)

> [!IMPORTANT]
> The `cargo-rullst` CLI must **never** be allowed to grow into a monolithic `main.rs`. Once the file exceeds ~1000 lines, refactoring into the module structure below is **mandatory**. Mixing template strings, HTML, SQL schemas, spinner logic, and argument parsing in a single file is classified as a **critical architecture violation**.

### 10.1. Required Module Structure

The `cargo-rullst` source directory must be organized as follows:

```text
cargo-rullst/
├── src/
│   ├── main.rs               # ≤ 80 lines: Entry point only. Dispatches to cli or ui.
│   ├── cli.rs                # Clap structs, Commands enum, argument definitions.
│   ├── ui/                   # Everything visual: banners, spinners, menus, boxes.
│   │   ├── mod.rs
│   │   ├── banner.rs         # ASCII art, version display, neon color palette.
│   │   └── components.rs     # with_spinner(), prompt_select(), boxed_output().
│   ├── generators/           # Scaffold logic: writes files to disk on user's project.
│   │   ├── mod.rs
│   │   ├── controller.rs     # create_new_controller()
│   │   ├── model.rs          # create_new_model()
│   │   ├── migration.rs      # create_new_migration(), regenerate_migrations_mod()
│   │   ├── project.rs        # create_new_project() — main wizard
│   │   ├── auth.rs           # scaffold_auth_system()
│   │   ├── billing.rs        # scaffold_billing_system()
│   │   ├── desktop.rs        # scaffold_omni_system() / run_omni_app()
│   │   └── foundry.rs        # scaffold_foundry_config() / run_foundry_deploy()
│   └── blueprints/           # Blueprint template definitions (NOT inline strings).
│       ├── mod.rs
│       ├── blank.rs          # Blank Starter template files
│       ├── lms.rs            # LMS / Course Platform template files
│       ├── saas.rs           # SaaS Starter template files
│       └── blog.rs           # Blog / Content System template files
```

### 10.2. The `main.rs` Purity Rule

`main.rs` is the **maestro**, not a musician. It must contain only:
1. Crate-level `#![allow(...)]` attributes.
2. Module declarations (`pub mod cli; pub mod ui; pub mod generators; pub mod blueprints;`).
3. The `fn main()` function itself — which reads arguments and dispatches.
4. Zero business logic, zero template strings, zero file I/O.

```rust
// ✅ CORRECT main.rs pattern
fn main() -> Result<(), Box<dyn std::error::Error>> {
    trigger_background_update_check();
    if std::env::args_trimmed().has_no_subcommand() {
        ui::show_interactive_dashboard()?;
    } else {
        cli::parse_and_run()?;
    }
    Ok(())
}
```

### 10.3. Template String Rules

**Never** embed large multi-line HTML, Rust, or SQL strings directly inside scaffold generator functions. Instead:

- **Option A (Preferred):** Use `include_str!()` to load `.rs.tmpl`, `.html`, or `.sql` files from a `templates/` directory compiled into the binary.
- **Option B:** Define typed constants or functions inside the `blueprints/` module (e.g., `blueprints::lms::course_model_rs()`) that return `&'static str` or `String`. These can be updated independently without touching generator logic.
- **Option C (Last Resort):** Use `r###"..."###` raw string literals (triple-hash delimiters) to prevent early termination when templates contain `"#` sequences (common in HTML attributes like `hx-target="#player-panel"`).

> [!WARNING]
> Using `r#"..."#` (single-hash) raw strings for HTML templates **will cause a compiler error** whenever the template contains `"#` (which is extremely common in HTML `id` attributes used with HTMX). Always use `r###"..."###` or move the template to the `blueprints/` module.

---

## 🎨 11. Rullst Blueprints Engine — Design Rules

Blueprints are pre-configured project archetypes generated by the `cargo rullst new` wizard. They must follow strict rules to guarantee quality, safety, and ease of maintenance.

### 11.1. Available Blueprints

| Blueprint ID | Name | Description |
|---|---|---|
| `0` | 📝 Blank Starter | Minimal HTMX reactive counter. Clean baseline. |
| `1` | 🎓 LMS / Course Platform | Courses + Lessons models, migrations with seed data, glassmorphic video player via HTMX. |
| `2` | 🛍️ SaaS Starter | Auth system + Stripe pricing panels + user dashboard. |
| `3` | 📰 Blog / Content System | Post model, auto-CMS via Nexus, glassmorphic press feed. |

### 11.2. Blueprint Scaffolding Rules

1. **Only during `new`:** Blueprint file injection (route wiring, `lib.rs`, `main.rs`) may **only** happen during `cargo rullst new`, when the project directory is freshly created and fully controlled. Never attempt to inject code into an existing user project.
2. **Isolated file generation:** Each blueprint generates a fully self-contained set of files. No cross-file regex or AST manipulation is permitted.
3. **Template sourcing:** All blueprint templates must live in `src/blueprints/<name>.rs` as typed functions, not inside `generators/project.rs`.
4. **Additive, not destructive:** Blueprints append to or create new files. They never delete or overwrite files created by a previous step.

### 11.3. Blueprint File Manifest

Every blueprint module (e.g., `blueprints::lms`) must expose a `FileManifest` — a list of `(relative_path, content)` pairs — via a public function:

```rust
// src/blueprints/lms.rs
pub fn file_manifest() -> Vec<(&'static str, String)> {
    vec![
        ("src/models/course.rs",    course_model()),
        ("src/models/lesson.rs",    lesson_model()),
        ("src/models/mod.rs",       "pub mod course;\npub mod lesson;\n".to_string()),
        ("src/controllers/lms_controller.rs", lms_controller()),
        ("src/pages/lms.rs",        lms_page()),
        // ...migrations, etc.
    ]
}
```

The generator iterates over this manifest and writes each file — no HTML or Rust templates inline.

---

## 🔐 12. Environment Variables & Third-Party Secrets

Any blueprint or scaffold that integrates a paid third-party service (Stripe, LemonSqueezy, Cloudinary, etc.) **must** generate a `.env` file with clear, commented placeholder values.

### 12.1. Required `.env` Template

```dotenv
# ─────────────────────────────────────────────────────────────
#  Rullst Application Environment Configuration
#  Generated automatically by cargo rullst new
# ─────────────────────────────────────────────────────────────

# ⚠️ SECURITY: This file must NEVER be committed to git.
# It is automatically added to .gitignore by the Rullst CLI.

# ── Database ──────────────────────────────────────────────────
DATABASE_URL=sqlite://db.sqlite3

# ── Application ───────────────────────────────────────────────
APP_KEY=GENERATE_A_RANDOM_32_CHAR_SECRET_HERE
APP_ENV=development

# ── Stripe Billing (replace with your real keys from stripe.com/dashboard) ──
# STRIPE_SECRET_KEY=sk_test_REPLACE_WITH_YOUR_SECRET_KEY
# STRIPE_WEBHOOK_SECRET=whsec_REPLACE_WITH_YOUR_WEBHOOK_SECRET
# STRIPE_PRICE_ID_MONTHLY=price_REPLACE_WITH_YOUR_PRICE_ID
```

### 12.2. `.gitignore` Auto-Protection Rules

The CLI **must** automatically append the following entries to `.gitignore` during project creation:

```gitignore
# Rullst: Environment & Secrets
.env
.env.*
!.env.example

# Rullst: Foundry Deployment Manifest (contains SSH keys and cloud credentials)
Foundry.toml
```

### 12.3. `.env.example` Companion File

Every project generated by `cargo rullst new` must also produce an `.env.example` file with all the same keys but with the placeholder values clearly documented. This file **is** committed to version control and serves as the public onboarding guide for new contributors.
