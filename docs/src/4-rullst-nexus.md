# Rullst Nexus: Explicit Admin CMS

**Rullst Nexus** is a server-rendered administrative CMS for explicitly
registered Rullst models.

`NexusModel` metadata defines the tables, fields, and widgets available in the
panel. Rullst builds CRUD, search, pagination, and batch routes from that
registration; it does not discover an arbitrary database schema automatically.

<a id="mobile-maintenance-in-1210-and-v13-unreleased"></a>

## Mobile maintenance in 12.1.0

The [Portfolio report, issues 3 and 4](https://github.com/Rullst/examples/blob/0182464b68a5adaf89fca65af8dd00142d78ad49/docs/portfolio-errors-found.md)
identified a drawer that could not be dismissed on small screens and a
desktop-only Portfolio layout. The changes shipped in **12.1.0** and are absent from
12.0.0:

- Nexus supplies a close button, click/touch backdrop, Escape and link dismissal,
  focus containment and return, and synchronized `aria-expanded` state. Closed
  mobile links are inert; the background is inert while the drawer is open.
  Desktop resizing clears that state. Without JavaScript, ordinary navigation
  stays visible. The drawer does not require HTMX or a CDN to dismiss.
- The Portfolio generator stacks its sections below 900px, tightens spacing
  below 640px, wraps long content and keeps project cards inside the viewport.
  Reduced-motion preferences stop decorative animations.

After the target release is published, update the application dependencies and
lockfile, then remove the temporary `nexus_mobile_patch` middleware that buffers
and rewrites Nexus HTML. Retaining it can introduce duplicate close controls,
backdrops and event handlers. Do not remove authentication, authorization, TLS,
CSRF or other security middleware along with that presentation-only patch.

Existing Portfolio projects own their generated `src/pages/home.rs`: updating
the CLI or dependency alone does **not** replace that file. Compare its styles
with a fresh Portfolio scaffold, apply the responsive CSS while preserving your
content/design, and remove conflicting old overrides. Verify narrow screens,
long names/URLs, keyboard focus, touch dismissal, desktop resizing and reduced
motion before redeployment. Other repositories and deployed Azure applications
are not modified by these framework changes.

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
    pub id: i32,
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

The edit form sends only the fields you change, so an edit never rewrites a
value its widget cannot show: NULL (shown as an empty `NULL` input), an enum
value that is not a registered option (kept selected but disabled), a date-time
with an offset (shown as text) or a value that cannot be decoded. Emptying a
number, relation, date, date-time, enum or JSON field stores NULL; emptying a
text, textarea, e-mail or URL field stores an empty string. A database
`NOT NULL` constraint therefore rejects clearing a required typed column.

A `password` field is never displayed: the list shows a fixed mask and the
edit form an empty input, and leaving it empty keeps the stored value. Nexus
writes a new value exactly as typed and does **not** hash it. Keep hash columns
`readonly` (or `hidden`) in Nexus and change them through an application flow
that hashes; a field named `password_hash` is hidden by the derive.

On an ORM model the derive also follows the `#[derive(Orm)]` field markers:

- `#[orm(skip)]` and `#[sqlx(skip)]` fields have no column and are omitted.
- `#[orm(encrypted)]` and `SecretString` fields are hidden, read-only
  `password` fields: Nexus never lists, searches, sorts, renders or writes
  them, so an edit cannot store plaintext in an encrypted column. Any other
  `#[nexus(kind/options)]` on them is a compile error.
- `#[orm(hidden)]` fields are hidden and read-only; only an explicit
  `#[nexus(kind = "password")]` exposes one, as a write-only `password` field.
- `#[orm(masked)]` fields default to the `password` widget; an explicit
  `#[nexus(kind = ...)]` deliberately shows them.

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

### Basic Auth failures and reverse proxies

Nexus counts failed Basic credentials per client bucket: one IPv4 address or
one IPv6 /64 of the `ConnectInfo` peer. Five failures in five minutes lock the
bucket for fifteen minutes. The unauthenticated `401` challenge that every
browser receives first is not a failure.

A locked bucket gets `429` without any credential check, so the lockout cannot
confirm a guessed password. The exception is a browser that already logged in
during this process: it received the `HttpOnly`, `Secure`
`rullst_nexus_known_client` cookie and keeps having its credentials checked.

Behind a TLS-terminating reverse proxy every client shares the proxy's address,
so one attacker can lock the bucket for everyone who has no known-client cookie
(new browsers, or all browsers after a restart). Configure Core's trusted-proxy
layer with the proxy's own network to give each client its own bucket:

```rust,ignore
Server::new(router).trusted_proxies(
    TrustedProxyConfig::new(["10.0.0.0/8"])?.trust_forwarded_proto(true),
)
```

List only the networks your proxies connect from: any host inside a listed
network can choose the client address. Forwarding headers from every other peer
are ignored. With `trust_forwarded_proto(true)`, an `X-Forwarded-Proto: https`
from that trusted peer also satisfies the Basic Auth TLS requirement, so a
separate `NexusVerifiedTls` middleware is unnecessary; enable it only when the
proxy overwrites that header. The same settings are available in `Rullst.toml`
as `[security] trusted_proxies`, `trusted_proxy_header` and
`trust_forwarded_proto`. See the
[security architecture](security-architecture.md#identity-and-network-trust)
for the resolution rules.

## Tenant-scoped administration

Use an explicit tenant column when a registered model contains tenant-owned
rows. The column must be a text, non-primary-key field. The derive makes it
hidden and read-only so browser form data cannot choose the tenant:

```rust
use rullst::db::{FromRow, Nexus, Orm};

#[derive(Debug, Clone, FromRow, Orm, Nexus)]
#[orm(table = "projects", tenant = "organization_id")]
pub struct Project {
    pub id: i32,
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

`#[derive(Nexus)]` reads only `table` (or its ORM alias `table_name`) and
`tenant` from a shared `#[orm(...)]` attribute. Other ORM options, such as
`tenant_column`, `policy`, `soft_delete(...)` or a relation's `foreign_key`, are
skipped. ORM tenant isolation (`tenant_column`) and the Nexus admin scope
(`tenant`) are separate options; declare both when both are wanted. Fields that
declare an ORM relation (`has_many`, `belongs_to`, ...) are not table columns
and do not appear in Nexus.

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

## Content Security Policy

Nexus pages load only same-origin assets from `/nexus/assets/` (`nexus.css`,
`nexus.js`, a vendored htmx 2.0.4 and the Rullst logo `rullst-logo.png`,
used as the brand mark and favicon) and contain no inline scripts, styles,
event-handler attributes or `hx-on` attributes. The default production CSP
applies to the panel unchanged, so there is no reason to add `'unsafe-inline'`,
`'unsafe-eval'` or a CDN to the application-wide `security.csp`. A custom
policy must keep `'self'` for scripts, styles and `connect-src`, and `data:`
for images. Nothing is requested from GitHub, unpkg or Google Fonts; the panel
uses system fonts.

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
