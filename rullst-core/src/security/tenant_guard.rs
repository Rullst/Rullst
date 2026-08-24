//! Multi-tenant isolation guard middleware and request extension context.

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Represents the active tenant context extracted from request headers or authentication tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantContext {
    /// The unique identifier of the tenant or organization.
    pub tenant_id: String,
}

impl TenantContext {
    /// Creates a new [`TenantContext`] with the given tenant identifier.
    pub fn new(tenant_id: impl Into<String>) -> Self {
        Self {
            tenant_id: tenant_id.into(),
        }
    }
}

/// Middleware that extracts tenant identification from standard HTTP headers (`X-Tenant-ID`, `X-Organization-ID`)
/// and injects [`TenantContext`] into request extensions for automated database multi-tenancy scoping.
#[cfg_attr(mutants, mutants::skip)]
pub async fn tenant_guard_middleware(mut req: Request, next: Next) -> Response {
    let tenant_id = req
        .headers()
        .get("X-Tenant-ID")
        .or_else(|| req.headers().get("X-Tenant-Id"))
        .or_else(|| req.headers().get("x-tenant-id"))
        .or_else(|| req.headers().get("X-Organization-ID"))
        .or_else(|| req.headers().get("X-Org-ID"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.trim().to_string());

    if let Some(id) = tenant_id {
        req.extensions_mut().insert(TenantContext::new(id));
    }

    next.run(req).await
}

/// Strict multi-tenant guard middleware that rejects any incoming request missing a valid tenant header with `400 Bad Request`.
#[cfg_attr(mutants, mutants::skip)]
pub async fn strict_tenant_guard_middleware(mut req: Request, next: Next) -> Response {
    let tenant_id = req
        .headers()
        .get("X-Tenant-ID")
        .or_else(|| req.headers().get("X-Tenant-Id"))
        .or_else(|| req.headers().get("x-tenant-id"))
        .or_else(|| req.headers().get("X-Organization-ID"))
        .or_else(|| req.headers().get("X-Org-ID"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.trim().to_string());

    match tenant_id {
        Some(id) if !id.is_empty() => {
            req.extensions_mut().insert(TenantContext::new(id));
            next.run(req).await
        }
        _ => (
            StatusCode::BAD_REQUEST,
            "Missing required multi-tenant header (e.g. X-Tenant-ID)",
        )
            .into_response(),
    }
}
