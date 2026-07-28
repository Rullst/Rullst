# Rullst Nexus ⚙️

`rullst-nexus` is the auto-generated, dark-mode Content Management System (CMS) and Admin Panel for the Rullst Framework. It dynamically inspects your Rust structs and database schema to build a full-featured admin interface instantly.

## ✨ Features

- **Zero-Config Admin Panel:** Generates complete CRUD (Create, Read, Update, Delete) interfaces directly from your `rullst-orm` models.
- **Wasm Islands:** Uses Rust-based WebAssembly for hyper-fast, SPA-like interactions without writing a single line of JavaScript.
- **Rich Media Support:** Built-in drag-and-drop file uploads, Markdown editors, and image previews for `Text` and `Blob` columns.
- **Relational Awareness:** Automatically understands and provides dropdowns or multi-selects for `HasMany` and `BelongsTo` relationships.
- **Role-Based Protection:** Native integration with `rullst-auth` ensures only authenticated Administrators can access the Nexus dashboard.

## 🚀 Quickstart

Add `rullst-nexus` to your project:

```bash
cargo add rullst-nexus
```

### Exposing Nexus

You can easily mount Nexus onto an existing Router. By default, it inspects the global `Orm` pool to map all registered tables.

```rust
use rullst::{Router, Server};
use rullst_nexus::NexusLayer;
use rullst_orm::Orm;
use rullst_auth::{AuthLayer, SessionStore};

#[tokio::main]
async fn main() {
    let pool = Orm::pool();
    let session_store = SessionStore::postgres(pool.clone());
    
    let admin_app = Router::new()
        // Ensure only admins can access Nexus
        .layer(AuthLayer::new(session_store).require_role("admin"))
        // Mount Nexus
        .nest("/admin", NexusLayer::new(pool).into_router());

    Server::new().route("/", admin_app).run().await;
}
```

Now, navigate to `http://localhost:3000/admin` to manage your application data!

## 🔐 Security Audit

`rullst-nexus` generates UI forms dynamically. To protect against CSRF (Cross-Site Request Forgery) and XSS (Cross-Site Scripting), it utilizes automatic CSRF token injection in forms and strictly escapes all user-generated content rendered in the Admin Panel tables. Modifying database records through Nexus passes through the same `rullst-orm` validations as your public API.

## 📚 Documentation

For advanced usage, customizing the Admin Panel's CSS, and overriding default form widgets, please visit the **[Rullst Book](https://rullst.github.io/Rullst/book/index.html)**.
