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

Then select an explicit access policy in your routing file (usually `src/lib.rs`
or `src/main.rs`) and mount the resulting router:

```rust
let nexus_auth =
    rullst::nexus::NexusAuthPolicy::local_development_or_basic_from_env()?;
let nexus = rullst::nexus::Nexus::new()
    .with_auth_policy(nexus_auth)
    .with_brand("SaaS Admin")
    .register::<models::user::User>()
    .try_build()?;

// ... and add it to the final router:
let router = router.nest_axum("/nexus", nexus);
```

The helper is intentionally asymmetric: debug builds allow only requests whose
`ConnectInfo` peer is loopback; release builds load and validate
`NEXUS_ADMIN_USERNAME` and `NEXUS_ADMIN_PASSWORD`. Missing connection metadata
is denied, and `APP_ENV` cannot turn credential-free access on in a release
binary. Applications can call `basic_from_env()` directly in debug when testing
the production authentication flow.

## 👤 Example: Dynamic Profile Settings in Blueprints

Starter blueprints like **Portfolio** use Nexus reflection to expose single-row or multi-row site configuration settings (such as developer name, title, bio, email, personal website, avatar photo, and social links).

```rust
use rullst::db::{Orm, FromRow, Nexus};

#[derive(Debug, Clone, FromRow, Orm, Nexus)]
#[orm(table = "profile")]
pub struct Profile {
    pub id: i32,
    pub name: String,
    pub title: String,
    pub subtitle: String,
    pub email: String,
    pub website: String,
    pub avatar_url: String,
    pub github_url: String,
    pub linkedin_url: String,
}
```

When registered in Nexus:
```rust
let nexus_auth =
    rullst::nexus::NexusAuthPolicy::local_development_or_basic_from_env()?;
let nexus = rullst::nexus::Nexus::new()
    .with_auth_policy(nexus_auth)
    .with_brand("Portfolio Admin")
    .register::<models::profile::Profile>()
    .try_build()?;
```

Administrators can edit the developer profile fields directly at `/nexus`. Changes to name, title, bio, email, website, avatar image, and social URLs immediately update the live portfolio website without requiring code changes or server redeployment!

## Benefits of Nexus

1. **Small Front-end Surface:** Nexus renders responsive tables, forms and
   actions with server-side HTML and HTMX.
2. **Fail-closed Construction:** Nexus cannot build without a selected access
   policy. Being in the same binary is not itself a security guarantee; the
   application still owns TLS, trusted proxies, roles, ownership, field policy
   and database permissions.
3. **Server-side Field Policy:** `hidden` and `readonly` metadata improve the UI,
   while authorization and write restrictions are also enforced on the server.

In a generated debug application, open `/nexus` from the same machine. In a
release deployment, configure strong unique credentials and the verified TLS
boundary before exposing the route.
