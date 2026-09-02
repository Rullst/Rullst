# Rullst Nexus: Explicit Admin CMS

**Rullst Nexus** is a server-rendered administrative CMS for explicitly
registered Rullst models.

`NexusModel` metadata defines the tables, fields, and widgets available in the
panel. Rullst builds CRUD, search, pagination, and batch routes from that
registration; it does not discover an arbitrary database schema automatically.

## Derive and register a model

The `Nexus` derive generates `NexusModel` metadata for named-field structs. It
infers booleans, numbers, dates and ordinary text; semantic widgets that Rust's
type alone cannot reveal are selected explicitly:

```rust
use rullst::db::{FromRow, Nexus, Orm};

#[derive(Debug, Clone, FromRow, Orm, Nexus)]
#[orm(table = "users")]
#[nexus(label = "Users", icon = "👥")]
pub struct User {
    pub id: i64,
    pub name: String,
    #[nexus(kind = "email")]
    pub email: String,
    #[nexus(kind = "textarea", label = "Biography")]
    pub bio: String,
    #[nexus(kind = "enum", options = "invited, active, suspended")]
    pub status: String,
    pub is_active: bool,
}
```

`id` is the default primary key. Use `#[nexus(primary_key)]` on a field or
`#[nexus(primary_key = "uuid")]` on the struct for another key. Field options
also include `label`, `hidden`, `readonly`, and the `text`, `textarea`, `email`,
`url`, `number`, `boolean`, `date`, `datetime`, `password`, `json`, and `enum`
widget kinds. Implementing `NexusModel` manually remains available when an
application needs metadata that cannot be derived.

Then select an explicit access policy in your routing file (usually `src/lib.rs`
or `src/main.rs`) and mount the resulting router:

```rust,ignore
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
is denied, and neither `RULLST_ENV` nor legacy `APP_ENV` can turn credential-free access on in a release
binary. Applications can call `basic_from_env()` directly in debug when testing
the production authentication flow.

## Tenant-scoped administration

Use an explicit tenant column when a registered model contains tenant-owned
rows. The column must be a text, non-primary-key field. The derive makes it
hidden and read-only so browser form data cannot choose the tenant:

```rust
use rullst::db::{FromRow, Nexus, Orm};

#[derive(Debug, Clone, FromRow, Orm, Nexus)]
#[orm(table = "projects", tenant = "organization_id")]
pub struct Project {
    pub id: i64,
    pub organization_id: String,
    pub name: String,
    pub active: bool,
}
```

Authentication middleware must resolve membership and install a trusted
`rullst::security::TenantContext`. Do not construct it directly from
`X-Tenant-ID`, a query parameter or another client assertion. Nexus applies the
exact scope to list/search/edit/create/update/delete and batch routes; missing
context denies a scoped model. A model without the attribute remains global by
design.

## Require transaction-coupled audit

Install the fixed audit schema as an explicit deployment step, then enable the
policy on the panel:

```rust,ignore
rullst::nexus::create_nexus_audit_table().await?;

let nexus = rullst::nexus::Nexus::new()
    .with_auth_policy(nexus_auth)
    .register::<Project>()
    .with_required_audit()
    .try_build()?;
```

Each successful mutation and its minimized `rullst_nexus_audits` row commit in
one database transaction. Audit failure rolls the mutation back. The record
contains actor, optional tenant, table/action, optional known key, affected-row
count, committed outcome, optional bounded request ID, timestamp and format
version. `verify_nexus_audit_table()` checks deployment readiness and
`recent_nexus_audits()` reads at most 1,000 newest rows, optionally tenant
filtered; the application must authorize that export separately.

The table is neither append-only nor protected from a database administrator.
It does not persist denied attempts, and automatically assigned create keys are
not recovered uniformly across all supported SQL dialects. Protect database
permissions and send records to an independently operated immutable sink when
that property is required.

## 👤 Example: Dynamic Profile Settings in Blueprints

Starter blueprints like **Portfolio** use explicit Nexus metadata to expose
single-row or multi-row site configuration settings (such as developer name,
title, bio, email, personal website, avatar photo, and social links).

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
```rust,ignore
let nexus_auth =
    rullst::nexus::NexusAuthPolicy::local_development_or_basic_from_env()?;
let nexus = rullst::nexus::Nexus::new()
    .with_auth_policy(nexus_auth)
    .with_brand("Portfolio Admin")
    .register::<models::profile::Profile>()
    .try_build()?;
```

Administrators can edit the registered profile fields at `/nexus`. A blueprint
that reads those fields on each request can show the persisted values without a
code change or redeployment; cache policy remains application-owned.

Batch deletion is available for every registered model. Batch deactivation is
shown only when the model declares a writable Boolean `is_active` or `active`
field; Nexus never guesses which arbitrary status value means inactive.

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
