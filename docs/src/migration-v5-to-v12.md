# Migration guide: v5 to v12

Baseline: the repository tag `v5.0.0` and the compatible v5 line. This is a
multi-major migration. Use a branch and follow the common
[safe upgrade procedure](migration-v12.md) before applying these changes.
The [assisted upgrade tutorial](tutorials/36-assisted-framework-upgrades.md)
shows the complete CLI transaction and recovery flow.

## 1. Replace the dependency graph

The v5 facade had no default features, kept several modules inside `rullst`,
depended on `rullst-orm = 6.1.1`, and used `rullst-connect = 11`. V12 releases
all framework packages in one version train and enables `orm` plus
`queue-sqlite` by default.

To preserve the old opt-in behavior, start explicitly:

```toml
[dependencies]
rullst = {
    version = "12.0.0-rc.1",
    default-features = false,
    features = ["orm", "queue-sqlite"]
}
```

Add only the domain features the application actually uses. Consult the
[feature matrix](feature-matrix.md); do not copy `--all-features` into a
production manifest. Remove independently pinned old Rullst package versions or
move every direct package to the same v12 version.

Important feature changes include:

- `auth`, `ai`, `capital`, `nexus`, `studio`, `security`, and `iot` now select
  real optional crates rather than empty facade markers;
- `mail` selects HTTP/offline transports without SMTP; `mailer` remains a
  compatibility alias for `mail-smtp` when Lettre SMTP is required;
- `redis` selects both queue and cache Redis boundaries;
- `strict-postgres`, `strict-mysql`, and `strict-sqlite` select concrete ORM
  database types. Select at most one in an application.

## 2. Move from attribute-style routing

The v5 README demonstrated attribute-style `#[routes]` registration. V12's
supported registration surface is the explicit `routes!` macro. The deprecated
`#[route]` compatibility marker does not register a route.

```rust,no_run
use rullst::{Server, response::Html, routes};

async fn home() -> Html<&'static str> {
    Html("Hello from v12")
}

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = routes![get("/" => home)];
    Server::new(app).run(3000).await?;
    Ok(())
}
```

Direct Axum escape hatches remain supported through `rullst::web::axum` or a
normal direct dependency. The upgrade command does not rewrite those imports;
its versioned scanner reports the known v5 routing/server markers for manual,
semantic conversion.

## 3. Review ORM and migrations

- Replace the old independently pinned ORM with v12.
- Decide between SQLx `Any` and one concrete `strict-*` backend.
- Run schema migrations on a disposable restored database first.
- Recheck raw SQL parameterization, tenant predicates, ownership guards, query
  limits, pool timeouts, and rollback behavior.
- Never assume that a successful compile proves an old migration is reversible.

## 4. Rebuild administrative and security boundaries

Nexus no longer mounts as an implicitly open admin router. New code should use
`Nexus::try_build()` with either validated production authentication or
`LocalNexusAccess::loopback_only()` in a debug build. Basic Auth requires a
verified TLS boundary in production.

Tenant-owned Nexus models should add an explicit text tenant column to their
derive metadata and install `TenantContext` only after authenticated membership
resolution. Models without this metadata remain global administrator models.
If `with_required_audit()` is enabled, run
`create_nexus_audit_table()` as a deployment migration first; a missing table
intentionally rolls mutations back.

Studio must remain a separate debug-only loopback service. Rebuild the server
middleware order using the v12 production baseline, then add application
authorization and the extended `rullst-security` layers explicitly where used.
Re-test CSRF, CORS, webhook signatures, request limits, IDOR, and cross-tenant
denials.

## 5. Validate provider migrations

Empty or `mock_*` credentials intentionally use deterministic offline behavior.
They are not proof that mail, OAuth, billing, AI, or storage providers work
live. Validate each configured provider in a non-production account and keep
unsupported fiscal, transport, and hardware capabilities fail-closed.

## Completion gate

Run at minimum:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Also test the application's actual feature selection without `--all-features`,
database restore/migration/rollback, Nexus and Studio network exposure, and a
production-profile smoke build.
