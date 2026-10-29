# Rullst Nexus: Explicit Admin CMS

**Rullst Nexus** is a server-rendered administrative CMS for explicitly
registered Rullst models.

`NexusModel` metadata defines the tables, fields, and widgets available in the
panel. Rullst builds CRUD, search, pagination, and batch routes from that
registration; it does not discover an arbitrary database schema automatically.

## Derive and register a model

The `Nexus` derive generates `NexusModel` metadata for named-field structs. It
infers booleans, integers (`FieldKind::Integer` bounded by the Rust type, with
`u64`/`usize` capped at `i64::MAX`, so a fraction or out-of-range value is
rejected), floating-point numbers, dates (`chrono` `NaiveDate`; `DateTime<Utc>` and
`NaiveDateTime` under any path spelling, optional or not) and ordinary text;
semantic widgets that Rust's type alone cannot reveal are selected explicitly:

```rust
use rullst::db::{Nexus, Orm};

#[derive(Debug, Clone, Orm, Nexus)]
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
with an offset (shown as text), a text, e-mail or URL value with line breaks
or other control characters (shown read-only; declare `kind = "textarea"` to
edit multi-line text) or a value that cannot be decoded. Emptying a
number, relation, date, time, date-time, enum or JSON field stores NULL; emptying a
text, textarea, e-mail or URL field stores an empty string. A database
`NOT NULL` constraint therefore rejects clearing a required typed column.

### Dates and times

Each date and time field is parsed by its kind and written as a typed value, so
native `DATE`, `TIME`, `TIMESTAMP` and `TIMESTAMPTZ` columns accept it as well
as text columns. A malformed value is a `422` validation error that names the
field.

| Kind (derived from) | Input | Accepted value | Stored in a text column |
| :--- | :--- | :--- | :--- |
| `date` (`NaiveDate`) | `date` | `YYYY-MM-DD`, year 0001 to 9999 | `YYYY-MM-DD` |
| `time` (`NaiveTime`) | `time` | `HH:MM`, `HH:MM:SS` or with a fraction of up to 9 digits | `HH:MM:SS[.fraction]` |
| `datetime` (`NaiveDateTime`) | `datetime-local` | a local `YYYY-MM-DDTHH:MM[:SS[.fraction]]` (or a space for `T`) | `YYYY-MM-DD HH:MM:SS[.fraction]` |
| `datetime_utc` (`DateTime<Utc>`) | `datetime-local`, labelled with the panel's time zone, `(UTC)` by default | as `datetime`, read as a wall time in the panel's time zone; `Z`, `±HH:MM` or `±HH` is converted to UTC | RFC 3339, `2026-10-15T09:30:00+00:00` |

A `datetime-local` input has no time zone, so a `datetime_utc` field shows and
accepts wall times in the panel's time zone, which is UTC unless the
application selects another with `Nexus::with_time_zone` (v13):

```rust,ignore
use rullst::nexus::{Nexus, NexusTimeZone};

// A fixed offset (no daylight-saving changes), in minutes east of UTC.
let nexus = Nexus::new().with_time_zone(NexusTimeZone::fixed_offset(-180)?);
// An IANA zone with its daylight-saving rules: the `nexus-tz` feature
// (`rullst-nexus/tz`), which uses `jiff` like Core's `scheduler-tz`.
let nexus = Nexus::new().with_time_zone(NexusTimeZone::named("America/Sao_Paulo")?);
```

The label names the zone (`Starts at (America/Sao_Paulo)`). An administrator
in São Paulo enters 09:30 and Nexus stores `12:30:00+00:00`; the stored
instant is shown converted to the zone whatever offset the database reports
(PostgreSQL's `+00`, an RFC 3339 offset, or a MySQL/MariaDB `DATETIME` holding
UTC). In a zone with daylight-saving changes, a wall time that a gap skips
(clocks move forward) does not exist and answers `422` naming the zone, and a
wall time that a fold repeats (clocks move back) is read as its first, earlier
instant. A stored instant at the later occurrence of a repeated wall time is
shown in a text input as its UTC value (`2026-11-01T06:30:00Z`), so saving the
form unchanged keeps it; type an offset (`2026-11-01T01:30-05:00`) to choose
that occurrence. A value either picker cannot hold exactly (finer than
milliseconds, or a wall time outside the years 0001 to 9999) is shown in a text
input. 13.0.0-alpha.2 always used UTC.

The `datetime` form of a text column is the text `CURRENT_TIMESTAMP` and SQLx
write, so text comparisons and ordering agree with application rows. A
`datetime` value with an offset can only come from a text column and is stored
as entered.

In the list, a NULL or undecodable number, relation or Boolean shows a `NULL`
or `unreadable` marker rather than `0` or `No`, and a row whose key is NULL or
cannot be decoded exactly has no batch checkbox or edit/delete actions.

Opening the edit form of a missing, other-tenant or misspelled key returns
`404` (and a failed query `500`) instead of an empty editable form. The form
reads only the registered visible, non-password columns.

Other form values are bound as text. PostgreSQL has no assignment cast from text,
so there Nexus writes `number` and `integer` values through `NUMERIC`, relation values that
are canonical integers (or empty) through `BIGINT`, Booleans as untyped
`'0'`/`'1'` literals, and dates and times (and their NULL) through `DATE`,
`TIME`, `TIMESTAMP` or `TIMESTAMPTZ` casts: integer, numeric, floating-point,
`BOOLEAN` and date/time columns, the `INTEGER` columns of `Blueprint::boolean`
and text columns all accept them. MySQL/MariaDB receive a `datetime_utc` value
as its UTC wall time, which their `DATETIME` columns hold. Other kinds are
written as text, so keep editable JSON and enum values in text columns; native
`JSONB`, `UUID` or enum columns are not supported by Nexus. Lists and edit
forms read date and time fields as text (`CAST(... AS TEXT)` on PostgreSQL,
`CAST(... AS CHAR)` on MySQL/MariaDB), so native columns, such as the
`timestamps()` columns of a strict driver pool, display.

The panel addresses records under `/nexus/table/{table}/record/{key}` (with
`/edit` for the form), so a key named `new`, `search` or `batch` never collides
with an action route. The older `/nexus/table/{table}/{key}` routes remain for
other keys.

Record keys follow the registered primary-key kind: a `number`, `integer` (or
relation) key must be a canonical integer, so `+1`, `01` or `1e3` name no record, and any
other kind is compared as text, even when it looks numeric.

### Composite primary keys

A table keyed by several columns, such as `lesson_progress (user_id,
lesson_id)`, lists them all, in key order:

```rust,ignore
#[derive(Debug, Clone, Orm, Nexus)]
#[orm(table = "lesson_progress")]
#[nexus(primary_key = "user_id, lesson_id")]
pub struct LessonProgress {
    pub user_id: i32,
    pub lesson_id: i32,
    pub percent: i32,
}
```

Annotating each key field with `#[nexus(primary_key)]` declares the same key
(in field order); a manual `NexusModel` returns the columns from
`nexus_pk_columns()`. A key has at most 8 columns, each a registered field
other than the tenant column. Every key column is entered on create (even an
integer one), read-only on edit, and part of every read, update, delete and
batch predicate.

A record is addressed by its key values in key order, each percent-encoded
(every byte except ASCII letters, digits and `-_.~`) and joined by `,`: user 7
and lesson `intro` are `7,intro`, and a text value `a,b` becomes `a%2Cb`. The
URL encodes that address once more (`/nexus/table/lesson_progress/record/7%2Cintro`),
and the audit records it as the record key. Only the canonical spelling with
one value per key column names a record; integer columns still need canonical
integers.

Search matches the typed text literally (`%` and `_` are not wildcards) in the
visible text, textarea, e-mail and URL columns. It is case-insensitive on
PostgreSQL (`ILIKE`), ASCII case-insensitive on SQLite and follows the column
collation on MySQL/MariaDB. Live search keeps the current sort, starts again at
page 1, rebuilds the sort and pagination links for the new query and records it
in the URL, so saving a record refreshes the same view. A model without such a
visible column shows no search box, and a search query sent to it lists no rows
(with a note) instead of every row.

The list sorts only by the record key and visible, non-`password` columns; a
`sort_by` naming a hidden or `password` column orders by the key instead.

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
`#[nexus(primary_key = "uuid")]` on the struct for another key, and several
annotated fields or a comma-separated list for a
[composite key](#composite-primary-keys). An annotated field is the key even
when the struct also has an `id` field, which then stays an ordinary column;
annotations that differ from the struct-level list are a compile error. A
single integer key is treated as database-generated, so it is hidden and
read-only; any other key (a UUID or code) is listed and entered in the create
form, and stays read-only on edit.
Field options
also include `label`, `hidden`, `readonly`, and the `text`, `textarea`, `email`,
`url`, `number`, `boolean`, `date`, `datetime`, `time`, `datetime_utc`,
`password`, `json`, and `enum` widget kinds. A `hidden` field is left out of the list, search, sorting and the
create/edit forms, and a submitted value for it is rejected; `readonly` keeps a
field visible but rejects submitted values. Implementing `NexusModel` manually
remains available when an application needs metadata that cannot be derived.

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
`ConnectInfo` peer is loopback (an IPv4-mapped peer such as `::ffff:127.0.0.1`,
which a dual-stack `::` listener reports for IPv4 clients, counts as IPv4); release builds load and validate
`NEXUS_ADMIN_USERNAME` and `NEXUS_ADMIN_PASSWORD` from the process environment,
then the working directory's `.env`. Missing connection metadata
is denied, and neither `RULLST_ENV` nor legacy `APP_ENV` can turn credential-free access on in a release
binary. Applications can call `basic_from_env()` directly in debug when testing
the production authentication flow.

`Nexus::try_build()` returns a plain `axum::Router`. Mount it on a
`rullst::Router` with `.nest_axum("/nexus", nexus)`, as above:
`rullst::Router::nest` accepts only another `rullst::Router`. In a plain Axum
application, `axum::Router::nest("/nexus", nexus)` is the equivalent.
`rullst::Router` has no `with_state`; pass shared application state with
`.layer(Extension(state))` (`rullst::web::axum::Extension`) and read it in
handlers with the `Extension<T>` extractor, or mount an `axum::Router<S>` after
its own `with_state`; see
[sharing application state](tutorials/08-routing-and-middlewares.md#step-3-share-application-state).

#### Using the application's sign-in

An application with its own login can let it decide instead, so an
administrator who is already signed in is not asked to sign in again.
Implement `NexusAuthorizer`: Nexus calls it on every request, with the
request's method, URI, headers and extensions, before reading or rendering any
data.

```rust,ignore
use rullst::nexus::{NexusAccess, NexusAuthPolicy, NexusAuthorizer, NexusAuthorizerError};
use rullst::web::axum::http::request::Parts;

struct Admins;

impl NexusAuthorizer for Admins {
    async fn authorize(&self, request: &Parts) -> Result<NexusAccess, NexusAuthorizerError> {
        // Look the session up as your auth middleware does.
        let Some(user) = current_user(&request.headers).await? else {
            return Ok(NexusAccess::SignIn);
        };
        Ok(if user.is_admin {
            NexusAccess::allow(format!("user:{}", user.id))
        } else {
            NexusAccess::Deny
        })
    }
}

let nexus = rullst::nexus::Nexus::new()
    .with_auth_policy(NexusAuthPolicy::application(Admins))
    .register::<models::user::User>()
    .try_build()?;
```

| Answer | Response |
| :--- | :--- |
| `NexusAccess::allow(actor_id)` | the request proceeds; `actor_id` names the administrator in the mutation audit (1 to 255 bytes, no surrounding whitespace or control characters, else `503`) |
| `NexusAccess::SignIn` | a page request (`GET`/`HEAD` without `HX-Request`) is redirected (`303`) to `/login`; fragments and other methods get `401` with `HX-Redirect: /login`, which htmx and Nexus's script follow |
| `NexusAccess::Deny` | `403` |
| `Err(NexusAuthorizerError::new(reason))` | `503`; Nexus logs the reason and never admits the request |

Use `NexusAuthPolicy::Application(NexusApplicationAuth::new(Admins).with_login_path("/account/sign-in"))`
for another login page; it must be a local path (one leading `/`, at most 512
visible ASCII characters, no `\`), or `try_build()` returns
`NexusBuildError::InvalidLoginPath`. Because the check runs on every request, a
signed-out or demoted administrator loses access on the next one. The policy
does not need TLS evidence (your session cookie's `Secure` flag is the
application's concern); CSRF protection, the administrator role layer, tenant
scoping and the audit stay as with every policy.
`NexusAuthPolicy::local_development_or_application(Admins)` keeps the
credential-free loopback policy in debug builds.

The SaaS starter and the LMS example use it. Their `src/middlewares/nexus_access.rs`
admits the user of the epoch-checked login cookie whose `users.is_admin` column
is true, so signing out everywhere or a password change also ends Nexus
access. `is_admin` defaults to false and is declared `#[orm(db_default)]`:
ORM writes never touch it, so neither a form nor `save()` grants it, while
`User::query().where_eq("is_admin", true)` still filters on it. Grant the first administrator with your
database client, for example
`UPDATE users SET is_admin = TRUE WHERE email = 'you@example.com';`; that
administrator can then grant others from the Users model in Nexus. The
`users` table that `cargo rullst auth` generates has the same column, ready for
`NexusAuthPolicy::application`.

#### Serving the application without Nexus

Because the helper fails closed, a release build without
`NEXUS_ADMIN_USERNAME`/`NEXUS_ADMIN_PASSWORD` cannot build Nexus. The
Portfolio and ERP examples then start without `/nexus` (the ERP, whose back
office shares the Nexus policy, serves only `/health` and `/ready`) and print
one line naming the variables; a weak, placeholder or unreadable credential
still stops startup. In your own application, mount Nexus only when its policy
builds:

```rust,no_run
# #[cfg(feature = "nexus")]
# fn router() -> Result<rullst::Router, Box<dyn std::error::Error>> {
use rullst::nexus::{Nexus, NexusAuthPolicy, NexusBuildError};

let router = rullst::Router::new(); // your application's routes
let router = match NexusAuthPolicy::local_development_or_basic_from_env() {
    Ok(policy) => {
        let nexus = Nexus::new().with_auth_policy(policy).try_build()?;
        router.nest_axum("/nexus", nexus)
    }
    Err(NexusBuildError::MissingCredential { variable }) => {
        eprintln!(
            "Nexus is not mounted at /nexus: {variable} is not set; set \
             NEXUS_ADMIN_USERNAME and NEXUS_ADMIN_PASSWORD to enable it"
        );
        router
    }
    // A weak, placeholder or unreadable credential is still a startup error.
    Err(error) => return Err(error.into()),
};
# Ok(router)
# }
```

The debug policy also requires a local `Host` and, for unsafe methods, a
same-origin `Origin`. Its responses carry `Referrer-Policy: same-origin` unless
the application sets one; when a host layer imposes `no-referrer`, browsers send
`Origin: null`, which is accepted only with a single `Sec-Fetch-Site:
same-origin` header (page scripts cannot set it).

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
use rullst::db::{Nexus, Orm};

#[derive(Debug, Clone, Orm, Nexus)]
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
exact scope to list/search/edit/create/update/delete and batch routes (as a
binary-string comparison on MySQL/MariaDB, whose default collations ignore
case); missing context denies a scoped model. A model without the attribute remains global by
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
contains actor, optional tenant, table/action, optional known key (for a create,
the key the administrator entered; a database-generated key is absent), affected-row
count, committed outcome, optional bounded request ID, timestamp and format
version. A key that does not fit 1 to 256 bytes of unpadded text without
control characters is recorded as absent instead of blocking the change. `verify_nexus_audit_table()` checks deployment readiness and
`recent_nexus_audits()` reads at most 1,000 newest rows, optionally tenant
filtered; the application must authorize that export separately.

The table is neither append-only nor protected from a database administrator.
It does not persist denied attempts, and automatically assigned create keys are
not recovered uniformly across all supported SQL dialects. Protect database
permissions and send records to an independently operated immutable sink when
that property is required.

## 👤 Example: Dynamic Profile Settings

Applications like the **Portfolio** example (`examples/portfolio`) use explicit Nexus metadata to expose
single-row or multi-row site configuration settings (such as developer name,
title, bio, email, personal website, avatar photo, and social links).

```rust
use rullst::db::{Orm, Nexus};

#[derive(Debug, Clone, Orm, Nexus)]
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

Administrators can edit the registered profile fields at `/nexus`. An
application that reads those fields on each request can show the persisted values without a
code change or redeployment; cache policy remains application-owned.

Batch deletion is available for every registered model. Batch deactivation is
shown only when the model declares a writable (neither `hidden` nor
`readonly`) Boolean `is_active` or `active` field; Nexus never guesses which
arbitrary status value means inactive.

## AI assistant

The AI query assistant at `/nexus/chat` (and its htmx endpoint
`/nexus/chat/query`) answers questions about the registered models through
the providers `rullst-ai` reads from the environment (`GEMINI_API_KEY`,
`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `OLLAMA_HOST`, …), and with a built-in
offline schema helper when none is configured. Since 13.0.0-alpha.4 it is an
optional feature:

| Dependency | Feature that enables the assistant |
| --- | --- |
| `rullst` facade | `nexus-ai` (implies `nexus`); `nexus` alone leaves it out |
| `rullst-nexus` directly | `ai`, a default feature |

`cargo rullst new` enables `nexus-ai` when the SaaS starter is created with
AI (`--ai` or the wizard's AI option). Without the feature Nexus does not
depend on `rullst-ai` (a shorter first build), the panel shows no assistant
link, and `/nexus/chat` and `/nexus/chat/query` answer `404 Not Found` with a
notice naming the feature to enable; they stay behind the same access policy.
The security and telemetry pages report the assistant as not enabled. With
the feature, routes and behaviour are those of earlier releases:

```toml
rullst = { version = "=13.0.0-alpha.4", default-features = false, features = ["nexus-ai", "strict-sqlite"] }
```

## Content Security Policy

Nexus pages load only same-origin assets from `/nexus/assets/` (`nexus.css`,
`nexus.js`, a vendored htmx 2.0.4 and the Rullst logo `rullst-logo.png`,
used as the brand mark and favicon) and contain no inline scripts, styles,
event-handler attributes or `hx-on` attributes. The default production CSP
applies to the panel unchanged, so there is no reason to add `'unsafe-inline'`,
`'unsafe-eval'` or a CDN to the application-wide `security.csp`. A custom
policy must keep `'self'` for scripts, styles, images and `connect-src`.
Nothing is requested from GitHub, unpkg or Google Fonts; the panel
uses system fonts. htmx's history cache is disabled, so admin pages and open
edit forms are never snapshotted into origin-wide `localStorage`; Back reloads
the page from the server.

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

<a id="mobile-maintenance-in-1210-and-v13-unreleased"></a>

## Mobile maintenance in 12.1.0

This section is for applications still on 12.0.x; 12.1.0 and every 13.0
prerelease include these changes.

The [Portfolio report, issues 3 and 4](https://github.com/Rullst/examples/blob/0182464b68a5adaf89fca65af8dd00142d78ad49/docs/portfolio-errors-found.md)
identified a drawer that could not be dismissed on small screens and a
desktop-only Portfolio layout. The changes shipped in **12.1.0** and are absent from
12.0.0:

- Nexus supplies a close button, click/touch backdrop, Escape and link dismissal,
  focus containment and return, and synchronized `aria-expanded` state. Closed
  mobile links are inert; the background is inert while the drawer is open.
  Desktop resizing clears that state. Without JavaScript, ordinary navigation
  stays visible. The drawer does not require HTMX or a CDN to dismiss.
- The Portfolio generator (since 13.0.0-alpha.4 the
  [`examples/portfolio`](https://github.com/Rullst/Rullst/tree/main/examples/portfolio)
  application) stacks its sections below 900px, tightens spacing
  below 640px, wraps long content and keeps project cards inside the viewport.
  Reduced-motion preferences stop decorative animations.

Update the application dependencies and lockfile to 12.1.0 or later, then
remove the temporary `nexus_mobile_patch` middleware that buffers and rewrites
Nexus HTML. Retaining it can introduce duplicate close controls,
backdrops and event handlers. Do not remove authentication, authorization, TLS,
CSRF or other security middleware along with that presentation-only patch.

Existing Portfolio projects own their generated `src/pages/home.rs`: updating
the CLI or dependency alone does **not** replace that file. Compare its styles
with `examples/portfolio/src/pages/home.rs`, apply the responsive CSS while preserving your
content/design, and remove conflicting old overrides. Verify narrow screens,
long names/URLs, keyboard focus, touch dismissal, desktop resizing and reduced
motion before redeployment. Other repositories and deployed Azure applications
are not modified by these framework changes.
