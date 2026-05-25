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
