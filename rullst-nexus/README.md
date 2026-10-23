# Rullst Nexus

`rullst-nexus` is Rullst's authenticated administrative CMS for registered `rullst-orm`
models. It provides server-rendered CRUD views, server-side field policies, RBAC enforcement,
telemetry, and the optional AI assistant.

This README describes the v13 prerelease (`13.0.0-alpha.2`). Cargo never
selects a prerelease on its own, so pin it exactly, for example
`rullst = { version = "=13.0.0-alpha.3", default-features = false, features = ["nexus", "strict-postgres"] }`.

Since 12.1.0, Nexus includes the fixes for mobile drawer dismissal, keyboard
focus and no-JavaScript navigation. Applications upgrading from 12.0 that use
the temporary `nexus_mobile_patch` HTML-rewriting workaround must remove that
presentation patch, while preserving their security layers. See the
[mobile migration checklist](https://github.com/Rullst/Rullst/blob/main/docs/src/4-rullst-nexus.md#mobile-maintenance-in-1210).

When used through the `rullst` umbrella with its `orm` and `nexus` features,
`#[derive(Nexus)]` generates metadata for named-field models. Primitive widgets
are inferred (a Rust integer field becomes `FieldKind::Integer` bounded by its
type, with `u64`/`usize` capped at `i64::MAX`; `f32`/`f64` become `Number`);
semantic fields can use `#[nexus(kind = "textarea")]` or
`#[nexus(kind = "enum", options = "draft, published")]`. Models may also
implement `NexusModel` manually. Batch deactivation is exposed only for a
writable (neither `hidden` nor `readonly`) Boolean `is_active` or `active`
field; batch deletion is bounded to
1,000 explicitly selected records. `try_build()` rejects ambiguous or unsafe
registered metadata. Mutation forms are pair/byte bounded and reject unknown,
protected, duplicate or semantically invalid values before executing bound SQL.
Boolean inference is automatic; enum variants and multiline intent stay
explicit because a struct derive cannot inspect unrelated application types.

An update writes only the submitted fields, and the edit form submits only the
fields the administrator changed. Values a widget cannot show unchanged are never
rewritten by an unrelated edit: SQL NULL renders as an empty input marked `NULL`,
an unregistered enum value stays selected but disabled, a date-time with an offset
or more than millisecond precision (and any value a number, date, e-mail or URL
input would alter) is shown in a text input, a single-line value with line
breaks or other control characters is shown read-only in a text area (declare
`kind = "textarea"` to edit multi-line text), and an undecodable value renders
empty with a note. An emptied number, relation, date, time, date-time, enum or JSON
field is stored as NULL, never `''` (a new record omits it so the column default
applies); text, textarea, e-mail and URL fields store `''`.

Date and time fields are parsed by kind and bound as typed values, so native
`DATE`, `TIME`, `TIMESTAMP` and `TIMESTAMPTZ` columns accept them as well as
text columns: `Date` (`YYYY-MM-DD`, year 0001 to 9999, from `NaiveDate`),
`Time` (`HH:MM[:SS[.fraction]]`, a `time` input, from `NaiveTime`), `DateTime`
(a local `datetime-local` value with a `T` or space separator, from
`NaiveDateTime`) and `DateTimeUtc` (from `DateTime<Utc>`). A `datetime-local`
input has no zone, so a `DateTimeUtc` field shows the stored instant as a wall
time in the panel's time zone, reads a submitted value without an offset as a
wall time in that zone and is labelled with its name; one with `Z`, `±HH:MM`
or `±HH` is converted. The zone is UTC unless `Nexus::with_time_zone` selects
a `NexusTimeZone::fixed_offset(minutes)` or, with the `tz` feature, an IANA
`NexusTimeZone::named("America/Sao_Paulo")` (v13). There, a wall time that a
daylight-saving gap skips answers `422`, and one that a fold repeats is read
as its earlier instant; the later instant is shown as its UTC value in a text
input so it round-trips. In a text column a local
date-time is stored as `YYYY-MM-DD HH:MM:SS[.fraction]`, the text that
`CURRENT_TIMESTAMP` and SQLx write, so text comparisons and ordering agree with
application rows, and a `DateTimeUtc` value as RFC 3339
(`2026-10-15T09:30:00+00:00`); a `DateTime` value with an offset can only come
from a text column and is stored as entered. API clients should send only the fields they intend to
change. A rejected value answers `422` with a plain-text reason, which the
panel shows as a toast. A `FieldKind::Number` field accepts any finite number,
while `FieldKind::Integer { min, max }` accepts only a whole number in that
range (stored without a `+` sign or leading zeros) and renders `step="1"` with
the bounds, so a fraction or out-of-range value never reaches an integer column.

In the list, a NULL or undecodable number, relation or Boolean shows a `NULL`
or `unreadable` marker rather than `0` or `No`, and a row whose key is NULL or
cannot be decoded exactly has no batch checkbox or edit/delete actions.

Opening the edit form of a missing, other-tenant or misspelled key returns
`404` (and a failed query `500`) instead of an empty editable form. The form
reads only the registered visible, non-password columns.

Other form values are bound as text. PostgreSQL has no assignment cast from text,
so there Nexus writes `number` and `integer` values through `NUMERIC`, relation values that
are canonical integers (or empty) through `BIGINT`, Booleans as untyped
`'0'`/`'1'` literals and dates and times (including NULL) through `DATE`,
`TIME`, `TIMESTAMP` or `TIMESTAMPTZ` casts: integer, numeric, floating-point,
`BOOLEAN`, date/time and text columns, and the `INTEGER` columns of
`Blueprint::boolean`, all accept them. MySQL/MariaDB receive a `DateTimeUtc`
value as its UTC wall time, which their `DATETIME` columns hold. Other kinds
are written as text, so keep JSON and enum values in text columns; native
`JSONB`, `UUID` or enum columns are not supported by Nexus.

The panel addresses records under `/nexus/table/{table}/record/{key}` (with
`/edit` for the form), so a key named `new`, `search` or `batch` never collides
with an action route. The older `/nexus/table/{table}/{key}` routes remain for
other keys.

Record keys follow the registered primary-key kind: a `number`, `integer` (or
relation) key must be a canonical integer, so `+1`, `01` or `1e3` name no record, and any
other kind is compared as text, even when it looks numeric.

A composite primary key, such as `lesson_progress (user_id, lesson_id)`, is
declared with `#[nexus(primary_key = "user_id, lesson_id")]`, with
`#[nexus(primary_key)]` on each key field, or by returning the columns from
`NexusModel::nexus_pk_columns()` (at most 8 registered columns, none the tenant
column). Every key column is entered on create, read-only on edit and bound in
every list, edit, update, delete and batch predicate. A record is addressed by
its values in key order, each percent-encoded and joined by `,` (`7,intro`; a
text value `a,b` becomes `a%2Cb`); only that canonical spelling with one value
per column names a record, and the audit records it as the key.

Search matches the typed text literally (`%` and `_` are not wildcards) in the
visible text, textarea, e-mail and URL columns. It is case-insensitive on
PostgreSQL (`ILIKE`), ASCII case-insensitive on SQLite and follows the column
collation on MySQL/MariaDB. Live search keeps the current sort, starts again at
page 1, rebuilds the sort and pagination links for the new query and records it
in the URL, so saving a record refreshes the same view. A model without such a
visible column shows no search box, and a search query sent to it lists no rows
(with a note) instead of every row.

The list sorts only by the record key and visible, non-`Password` columns. A
`sort_by` naming a hidden or `Password` column orders by the key instead, so a
crafted URL cannot reveal the relative order of values the panel never shows.

## Tenant-scoped CRUD and mutation audit

Models whose rows belong to one tenant may opt into an exact text-column scope.
The derive hides and protects that field, and Nexus obtains its value only from
the trusted `TenantContext` installed by application authentication middleware:

```rust,ignore
use rullst::db::{FromRow, Nexus, Orm};

#[derive(Debug, Clone, FromRow, Orm, Nexus)]
#[orm(table = "projects", tenant = "organization_id")]
struct Project {
    id: i64,
    organization_id: String,
    name: String,
    active: bool,
}
```

Every built-in list, search, edit, create, update, delete and batch operation for
that model includes the exact tenant predicate; on MySQL/MariaDB, whose default
collations ignore case, it compares binary strings so `Acme` never matches
`acme`. Create injects the trusted
tenant value; a submitted tenant field is rejected. A scoped model fails with
`403 Forbidden` when no `TenantContext` is present. Models without `tenant`
metadata deliberately remain global administrator models.

Nexus can also require one minimized mutation record in the same relational
transaction as each successful mutation:

```rust,ignore
rullst::nexus::create_nexus_audit_table().await?; // deployment/migration step

let nexus = rullst::nexus::Nexus::new()
    .register::<Project>()
    .with_auth_policy(policy)
    .with_required_audit()
    .try_build()?;
```

`rullst_nexus_audits` stores the authenticated Nexus actor, optional tenant,
table, action, optional known record key (for a create, the key the administrator
entered; a database-generated key is absent), affected-row count, committed outcome,
bounded correlation ID, timestamp and format version. A record key that does
not fit 1 to 256 bytes of unpadded text without control characters is recorded
as absent. An unavailable audit table rolls the data mutation back and returns
a generic error. Use
`verify_nexus_audit_table()` as a deployment check and
`recent_nexus_audits(limit, tenant)` for a bounded, separately authorized
export.

This is transaction-coupled evidence, not an append-only or tamper-evident audit
service: it is in the same database, records only committed mutations, and an
auto-generated create key may be absent. The host still owns identity and
membership policy, database permissions, retention, backup, replication,
failed-attempt telemetry and external immutable delivery.

## Secure mounting

Nexus is fail-closed: `try_build()` returns an error until an explicit access policy is selected.
The built-in Basic Auth policy rejects weak/example credentials, compares both credential fields in
constant time, and counts failed credentials per client bucket: one IPv4 address or one IPv6 /64
taken from the verified socket peer. Five failures within five minutes lock the bucket for fifteen
minutes. A request without a Basic `Authorization` header only receives the `401` challenge and is
never counted.

While a bucket is locked, credentials from an unknown client are not evaluated (`429`), so the
lockout cannot be used to test passwords. After a successful login Nexus sets an `HttpOnly`,
`Secure` cookie (`rullst_nexus_known_client`, random per process). A browser that presents it keeps
having its credentials checked during a lockout, so an attacker sharing its address cannot lock that
administrator out. The value changes on restart, and a first login during an active lockout still
waits for the lockout to expire.

Basic credentials are only encoding, not encryption. The middleware therefore accepts them only
when trusted deployment middleware inserts `NexusVerifiedTls`, or when Core's trusted-proxy layer
reported HTTPS from a trusted proxy peer (see below). The marker is a security assertion: never
create it solely because an untrusted request supplied `X-Forwarded-Proto` or another forwarding
header.

```rust
use axum::Extension;
use rullst_core::{Router, Server};
use rullst_nexus::{
    Nexus, NexusAuthPolicy, NexusVerifiedTls,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let policy = NexusAuthPolicy::basic_from_env()?;
    let nexus = Nexus::new()
        // .register::<User>()
        .with_auth_policy(policy)
        .try_build()?;

    // This example assumes a trusted reverse proxy has already terminated and required TLS.
    // Insert the marker only at that trusted boundary.
    let app = Router::new()
        .nest_axum("/nexus", nexus)
        .layer(Extension(NexusVerifiedTls::from_trusted_tls_termination()));

    Server::new(app).run(3000).await?;
    Ok(())
}
```

`try_build()` returns a plain `axum::Router`. Rullst's `Router` (re-exported as
`rullst::Router`) mounts it with `nest_axum`, because its own `nest` accepts
only another Rullst `Router`; in a plain Axum application use
`axum::Router::nest("/nexus", nexus)`. Rullst's `Router` has no `with_state`:
pass shared application state with `.layer(Extension(state))` and read it with
the `Extension<T>` extractor.

`local_development_or_basic_from_env()` fails closed: in a release build
without the credentials it returns `NexusBuildError::MissingCredential`, so an
application that propagates the error does not start. To serve the
application without Nexus until the credentials are configured, match that
variant and skip `nest_axum`, as the generated Blog, Portfolio and ERP starters
do; the
[Nexus guide](https://github.com/Rullst/Rullst/blob/main/docs/src/4-rullst-nexus.md#serving-the-application-without-nexus)
shows the pattern.

### The application's own sign-in

`NexusAuthPolicy::application(authorizer)` lets an application with its own
login decide, so a signed-in administrator does not sign in twice. The
authorizer implements `NexusAuthorizer` and is asked on every request, with the
request's method, URI, headers and extensions, before any data is read:

```rust,ignore
use axum::http::request::Parts;
use rullst_nexus::{NexusAccess, NexusAuthPolicy, NexusAuthorizer, NexusAuthorizerError};

struct Admins;

impl NexusAuthorizer for Admins {
    async fn authorize(&self, request: &Parts) -> Result<NexusAccess, NexusAuthorizerError> {
        let Some(user) = current_user(&request.headers).await? else {
            return Ok(NexusAccess::SignIn); // 303 to /login, or 401 + HX-Redirect
        };
        Ok(if user.is_admin {
            NexusAccess::allow(format!("user:{}", user.id)) // the audit actor
        } else {
            NexusAccess::Deny // 403
        })
    }
}

let policy = NexusAuthPolicy::application(Admins);
```

An authorizer error answers `503` and is logged; an actor ID that the audit
cannot store unchanged (empty, over 255 bytes, padded or with control
characters) also answers `503`. Signed-out page requests (`GET`/`HEAD` without
`HX-Request`) are redirected to `/login` with `303`; fragments and other
methods get `401` with an `HX-Redirect` header. Choose another page with
`NexusAuthPolicy::Application(NexusApplicationAuth::new(Admins).with_login_path("/sign-in"))`;
a path that is not a single-slash local path of at most 512 visible ASCII
characters fails `try_build()` with `NexusBuildError::InvalidLoginPath`. The
policy needs no TLS evidence; CSRF, the administrator role layer, tenant scope
and the audit are unchanged. `local_development_or_application(authorizer)`
uses loopback access in debug builds instead.

Set unique values for `NEXUS_ADMIN_USERNAME` and `NEXUS_ADMIN_PASSWORD` in the process environment
or the project's `.env`; the password must contain at least 16 characters. Rullst's server supplies `ConnectInfo<SocketAddr>`, which the Basic Auth
guard requires so a forged forwarding header cannot choose the rate-limit identity.
The Basic Auth guard also requires `NexusVerifiedTls` from trusted transport
integration; an `https` request URI or a forwarding header alone never proves TLS.

Behind a reverse proxy, the socket peer is the proxy, so every client shares one failure bucket:
anyone can lock it, and only browsers holding the known-client cookie keep access. Give each client
its own bucket with Core's trusted-proxy layer, listing only the proxy's own network:

```rust,ignore
Server::new(app).trusted_proxies(TrustedProxyConfig::new(["10.0.0.0/8"])?.trust_forwarded_proto(true))
```

Forwarding headers are read only from peers inside those networks; any host in a listed network can
choose the client address. `trust_forwarded_proto(true)` also accepts that proxy's
`X-Forwarded-Proto: https` as the TLS evidence above; enable it only when the proxy overwrites the
header. The `NexusVerifiedTls` path keeps working unchanged.

For local development only, debug builds can explicitly select
`NexusAuthPolicy::loopback_only(LocalNexusAccess::loopback_only())`. It still requires a verified
loopback socket peer (an IPv4-mapped peer such as `::ffff:127.0.0.1`, which a dual-stack `::`
listener reports for IPv4 clients, counts as IPv4), an unambiguous local `Host` authority, and a matching
`Origin` for unsafe methods, and is rejected in release builds. Non-browser
clients can read without `Origin`; local mutation requests must supply their
matching origin explicitly (for example, `Origin: http://localhost:3000` with
`Host: localhost:3000`). Present cross-origin headers are rejected on every method.
Its responses carry `Referrer-Policy: same-origin` unless the application sets
one, so browsers send the real origin on the panel's own requests. When a host
layer imposes `no-referrer`, browsers send `Origin: null`; the policy then
accepts the request only with a single `Sec-Fetch-Site: same-origin`, a header
page scripts cannot set, so documents of other origins or local ports stay
rejected.

The generated Blog, Portfolio and ERP starters use
`NexusAuthPolicy::local_development_or_basic_from_env()`: debug builds select
that loopback-only policy, while release builds require the validated
environment credentials above. The SaaS and LMS starters use
`local_development_or_application` with a generated authorizer that admits the
user of the epoch-checked login cookie whose `users.is_admin` is true.
Applications can always select a policy explicitly when testing a production
topology.

## Password fields

A field of kind `password` (`#[nexus(kind = "password")]` or
`FieldKind::Password`) is never shown: the list renders a fixed mask and does not
select or sort by the column, and the edit form renders an empty password input.
Leaving that input empty keeps the stored value. A non-empty value is written
exactly as typed. Nexus does not hash it and bypasses ORM model hooks, so a
column holding Argon2 or other credential hashes must be `readonly` (or
`hidden`) in Nexus and changed through an application flow that hashes, for
example with `rullst_auth::hash_password_async`, or by a database trigger.
The derive already hides a field named `password_hash`.

## Browser assets and Content Security Policy

The panel loads only same-origin files served by the Nexus router under
`/nexus/assets/`: `nexus.css`, `nexus.js`, a vendored htmx and the Rullst logo
`rullst-logo.png` (brand mark and favicon). Pages contain no
inline `<script>`/`<style>` blocks, no `on*`/`hx-on` handler attributes and no
`style` attributes, and they do not contact a CDN, Google Fonts or GitHub. The
default production CSP (`script-src 'self' 'nonce-…'; style-src 'self' 'nonce-…'`)
therefore runs Nexus unchanged; do not add `'unsafe-inline'`, `'unsafe-eval'` or a
CDN to `security.csp` for Nexus. A custom policy must keep `'self'` in
`script-src`, `style-src`, `img-src` and `connect-src`. htmx runs
with `allowEval`, `allowScriptTags` and `includeIndicatorStyles` disabled, and
with its history cache off (`historyCacheSize: 0`, `refreshOnHistoryMiss: true`):
admin pages and open edit forms are never snapshotted into origin-wide
`localStorage`, and Back reloads the page from the server.
The asset routes sit behind the same authentication policy as the panel.

AI assistant replies, from a provider or the built-in fallback, are untrusted.
Nexus keeps only an explicit text-formatting allowlist (paragraphs, lists, code,
tables, emphasis and links with `rel="noopener noreferrer"`). Scripts, event
handlers, unsafe URL schemes and every element that loads a resource by itself,
such as `img`, are removed, so a prompt-injected reply cannot render an image
beacon even where no CSP restricts `img-src`.

`assets/htmx-2.0.4.min.js` is the unmodified upstream
[`dist/htmx.min.js`](https://github.com/bigskysoftware/htmx/blob/b82cf843e47e575dd8c2ad8fee547d8e2c3bb87f/dist/htmx.min.js)
of htmx 2.0.4 (tag `v2.0.4`, commit `b82cf843e47e575dd8c2ad8fee547d8e2c3bb87f`),
the same bytes previously loaded from `unpkg.com/htmx.org@2.0.4`. Its
[Zero-Clause BSD license](https://github.com/bigskysoftware/htmx/blob/b82cf843e47e575dd8c2ad8fee547d8e2c3bb87f/LICENSE)
is kept as `assets/HTMX-LICENSE`. SHA-256
`e209dda5c8235479f3166defc7750e1dbcd5a5c1808b7792fc2e6733768fb447`; SRI
`sha384-HGfztofotfshcF7+8n44JQL2oJmowVChPTg48S+jvZoztPfvwD79OC/LTtG6dMp+`.
The file is outside Cargo dependency scanning: an update needs upstream
provenance, a license and digest review, and the Nexus CSP browser check.

## Security boundaries

Nexus includes CSRF protection, validates registered semantic form values, and
escapes record values and registered metadata rendered into admin pages. Applications
must still place the complete production server behind TLS, install Rullst's secure headers and WAF,
apply authorization/ownership policy to any custom routes mounted next to Nexus,
derive `TenantContext` from authenticated membership rather than request headers,
authorize audit exports, and keep the declared field metadata compatible with
the actual database schema.
