# Rullst Nexus

> **v12 development notice:** This README documents the unreleased v12 source.
> Use a path dependency from this checkout until an immutable v12 RC exists on
> crates.io.

`rullst-nexus` is Rullst's authenticated administrative CMS for registered `rullst-orm`
models. It provides server-rendered CRUD views, server-side field policies, RBAC enforcement,
telemetry, and the optional AI assistant.

When used through the `rullst` umbrella with its `orm` and `nexus` features,
`#[derive(Nexus)]` generates metadata for named-field models. Primitive widgets
are inferred; semantic fields can use `#[nexus(kind = "textarea")]` or
`#[nexus(kind = "enum", options = "draft, published")]`. Models may also
implement `NexusModel` manually. Batch deactivation is exposed only for a
writable Boolean `is_active` or `active` field; batch deletion is bounded to
1,000 explicitly selected records. `try_build()` rejects ambiguous or unsafe
registered metadata. Mutation forms are pair/byte bounded and reject unknown,
protected, duplicate or semantically invalid values before executing bound SQL.
Boolean inference is automatic; enum variants and multiline intent stay
explicit because a struct derive cannot inspect unrelated application types.

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
that model includes the exact tenant predicate. Create injects the trusted
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
table, action, optional known record key, affected-row count, committed outcome,
bounded correlation ID, timestamp and format version. An unavailable audit
table rolls the data mutation back and returns a generic error. Use
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
constant time, limits failures by verified socket peer, and locks peers after repeated failures.

Basic credentials are only encoding, not encryption. The middleware therefore accepts them only on
an HTTPS request or when trusted deployment middleware inserts `NexusVerifiedTls`. This marker is a
security assertion: never create it solely because an untrusted request supplied
`X-Forwarded-Proto` or another forwarding header.

```rust
use axum::{Extension, Router};
use rullst_core::Server;
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
        .nest("/nexus", nexus)
        .layer(Extension(NexusVerifiedTls::from_trusted_tls_termination()));

    Server::new(app).run(3000).await?;
    Ok(())
}
```

Set unique values for `NEXUS_ADMIN_USERNAME` and `NEXUS_ADMIN_PASSWORD`; the password must contain
at least 16 characters. Rullst's server supplies `ConnectInfo<SocketAddr>`, which the Basic Auth
guard requires so a forged forwarding header cannot choose the rate-limit identity.
The Basic Auth guard also requires `NexusVerifiedTls` from trusted transport
integration; an `https` request URI or a forwarding header alone never proves TLS.

For local development only, debug builds can explicitly select
`NexusAuthPolicy::loopback_only(LocalNexusAccess::loopback_only())`. It still requires a verified
loopback socket peer, an unambiguous local `Host` authority, and a matching
`Origin` for unsafe methods, and is rejected in release builds. Non-browser
clients can read without `Origin`; local mutation requests must supply their
matching origin explicitly (for example, `Origin: http://localhost:3000` with
`Host: localhost:3000`). Present cross-origin headers are rejected on every method.

Generated applications use
`NexusAuthPolicy::local_development_or_basic_from_env()`: debug builds select
that loopback-only policy, while release builds require the validated
environment credentials above. Applications can always select either policy
explicitly when testing a production topology.

## Security boundaries

Nexus includes CSRF protection, validates registered semantic form values, and
escapes record values and registered metadata rendered into admin pages. Applications
must still place the complete production server behind TLS, install Rullst's secure headers and WAF,
apply authorization/ownership policy to any custom routes mounted next to Nexus,
derive `TenantContext` from authenticated membership rather than request headers,
authorize audit exports, and keep the declared field metadata compatible with
the actual database schema.
