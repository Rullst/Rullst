# Tutorial 33: Auth-bound multi-tenancy

Rullst Core can select a tenant from a request hint only after application
authentication has inserted a trusted `TenantMembership`. A hostname, header,
query parameter or request body is never sufficient proof of membership.

## 1. Configure the selection layer

```rust,no_run
use axum::{Extension, Router, routing::get};
use rullst_core::{
    multitenant::{TenantConfig, TenantLayer, TenantStrategy},
    security::TenantContext,
};

async fn tenant_dashboard(Extension(tenant): Extension<TenantContext>) -> String {
    format!("Selected workspace: {}", tenant.tenant_id)
}

fn tenant_routes() -> Router {
    let selection = TenantLayer::new(TenantConfig::new(TenantStrategy::Subdomain));
    Router::new()
        .route("/dashboard", get(tenant_dashboard))
        .layer(selection)
}
```

An outer, application-owned authentication layer must first validate the
session/token and insert `TenantMembership::try_new(...)` from trusted identity
claims. Without that extension, `TenantLayer` returns `403 Forbidden`. If the
requested subdomain is not in the authenticated membership set, it also returns
403. In Axum, remember that subsequently added layers run first; test the final
middleware order in-process.

`TenantStrategy::Header` and `TenantStrategy::Parameter` are also available,
but they remain untrusted selection hints. Query parameters additionally leak
more easily through history, referrers and access logs. The built-in subdomain
parser selects the first label only for hostnames with at least three labels;
custom-domain ownership and trusted-proxy normalization remain application and
deployment work.

## 2. Bind every database query

Tenant selection does not rewrite arbitrary SQL or automatically add a tenant
predicate to every ORM query. Bind the authenticated tenant explicitly:

```rust,no_run
use rullst_core::security::TenantContext;
use sqlx::{FromRow, PgPool};

#[derive(FromRow)]
struct Invoice {
    id: i64,
    tenant_id: String,
    total_minor: i64,
}

async fn list_tenant_invoices(
    pool: &PgPool,
    tenant: &TenantContext,
) -> Result<Vec<Invoice>, sqlx::Error> {
    sqlx::query_as::<_, Invoice>(
        "SELECT id, tenant_id, total_minor FROM invoices WHERE tenant_id = $1",
    )
    .bind(&tenant.tenant_id)
    .fetch_all(pool)
    .await
}
```

Use database constraints and, where appropriate, database-native row-level
security as additional defense. Negative tests must prove that one tenant
cannot read, update or delete another tenant's records through every relevant
route, repository, background job and administrative surface.

## 3. Operational boundaries

- `TenantContext` is server-selected state, not a claim accepted from JSON.
- `current_tenant_id()` is task-local convenience; passing an explicit
  `TenantContext` to domain/storage APIs is easier to audit.
- `TenantCache`, `TenantStorage`, `TenantRealtime` and `TenantPresence` provide
  validated namespaces, but applications still own business authorization and
  remote provider policy.
- Cross-process membership updates, custom-domain verification, database
  policy, audit retention and incident response are not automatic framework
  guarantees.
