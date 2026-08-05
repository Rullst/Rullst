# Tutorial 33: Multi-Tenant B2B Enterprise Architecture 🏢

Build multi-tenant B2B applications with subdomains, custom domains, and database isolation using `rullst::multitenant`.

---

## 🛠️ Step 1: Configure Multi-Tenancy Middleware

```rust
use rullst_core::multitenant::{MultitenantLayer, TenantContext};
use axum::{Router, routing::get};

pub async fn tenant_dashboard(tenant: TenantContext) -> String {
    format!("Welcome Tenant [{}] to your isolated workspace!", tenant.subdomain)
}

pub fn app_router() -> Router {
    Router::new()
        .route("/dashboard", get(tenant_dashboard))
        .layer(MultitenantLayer::subdomain())
}
```

---

## 🔒 Step 2: Tenant Database Isolation

In database queries, scope models to the active tenant ID:

```rust
use crate::models::Invoice;

pub async fn list_tenant_invoices(tenant: TenantContext) -> Result<Vec<Invoice>, rullst_core::AppError> {
    let invoices = Invoice::where_clause("tenant_id = ?", vec![tenant.id]).await?;
    Ok(invoices)
}
```

---

## 💡 Key Takeaways
- Supports subdomain multitenancy (`acme.saas.com`) and custom domain mapping (`app.acme.com`).
- Prevents cross-tenant data leaks at the framework level.
