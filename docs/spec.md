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
│   ├── models/           # Active Record Models (rust-eloquent entities)
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
* **URL Paths:** Lowecase kebab-case (e.g. `/users`, `/user-profiles`).

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

### 3.3. Active Record ORM (`rust-eloquent`)

* **Model definition:**
  ```rust
  #[derive(Debug, Clone, FromRow, rust_eloquent::Eloquent)]
  #[eloquent(table = "table_name")]
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

---

## 🧱 5. Controller Architecture

Controllers handle business logic and HTTP responses.
* **Module Structure:** Each controller is a separate module inside `src/controllers/` (e.g., `users_controller.rs`).
* **Function Signatures:** Functions must be asynchronous and return a type that implements `axum::response::IntoResponse` (or `Result<impl IntoResponse, AppError>`).
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
