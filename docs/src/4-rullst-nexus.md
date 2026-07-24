# Rullst Nexus: Your Instant CMS

**Rullst Nexus** is a tightly coupled and dynamically generated Content Management System (CMS) for your Rullst project.

Instead of spending dozens of hours developing an administrative panel (CRUDs) for your database tables, Rullst Nexus reads the metadata from your database models (`structs`) and builds a stunning administrative interface (using Tailwind CSS and glassmorphism layouts) at the exact moment the application compiles.

## How it Works

All you need to do is implement the `NexusModel` trait on your ORM `struct`:

```rust
use rullst::nexus::{NexusModel, FieldMeta, FieldKind};

impl NexusModel for User {
    fn nexus_table() -> &'static str { "users" }
    fn nexus_label() -> &'static str { "Users" }
    fn nexus_icon() -> &'static str { "👥" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "name", label: "Name", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "email", label: "Email", kind: FieldKind::Text, hidden: false, readonly: false },
        ]
    }
}
```

And then, in your routing file (usually `src/lib.rs` or `src/main.rs`), you "hook up" the Nexus engine:

```rust
let nexus = rullst::nexus::Nexus::new()
    .with_brand("SaaS Admin")
    .register::<models::user::User>()
    .build();

// ... and add it to the final router:
let router = router.nest_axum("/nexus", nexus);
```

## Benefits of Nexus

1. **Zero Front-end Effort:** Nexus renders responsive tables, creation/edition modals, and delete buttons using HTML and HTMX without writing a single line of JS.
2. **Totally Secure:** Nexus lives *inside* the same compiled binary. There is no need for separate APIs or complex permissions to access the database; it uses the secure global connection pool.
3. **Highly Customizable:** The `hidden` and `readonly` flags in `FieldMeta` ensure you control exactly what the administrative team can see and modify.

To access it, open your browser at the `/nexus` route on your app.
