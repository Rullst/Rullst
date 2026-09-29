//! Multi-tenant isolation guard middleware and request extension context.

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Represents an active tenant selected by trusted authentication middleware.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TenantContext {
    /// The unique identifier of the tenant or organization.
    pub tenant_id: String,
}

impl TenantContext {
    /// Creates a validated [`TenantContext`] from a trusted identity claim.
    ///
    /// Identifiers are 1 to 128 bytes of ASCII letters, digits, `-`, `_`, `.`
    /// and `:`, and must not consist only of dots.
    pub fn try_new(tenant_id: impl Into<String>) -> Result<Self, TenantContextError> {
        let tenant_id = tenant_id.into();
        validate_tenant_id(&tenant_id)?;
        Ok(Self { tenant_id })
    }
}

/// Membership claims established by authentication middleware.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantMembership {
    tenant_ids: Vec<String>,
    default_tenant_id: Option<String>,
}

impl TenantMembership {
    /// Creates a validated membership set from authenticated claims.
    pub fn try_new<I, S>(tenant_ids: I) -> Result<Self, TenantContextError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut tenant_ids = tenant_ids
            .into_iter()
            .map(Into::into)
            .collect::<Vec<String>>();
        if tenant_ids.is_empty() {
            return Err(TenantContextError::EmptyMembership);
        }
        for tenant_id in &tenant_ids {
            validate_tenant_id(tenant_id)?;
        }
        tenant_ids.sort_unstable();
        tenant_ids.dedup();
        Ok(Self {
            tenant_ids,
            default_tenant_id: None,
        })
    }

    /// Selects a default tenant that must belong to this authenticated identity.
    pub fn with_default(
        mut self,
        tenant_id: impl Into<String>,
    ) -> Result<Self, TenantContextError> {
        let tenant_id = tenant_id.into();
        validate_tenant_id(&tenant_id)?;
        if !self.tenant_ids.iter().any(|allowed| allowed == &tenant_id) {
            return Err(TenantContextError::TenantNotInMembership(tenant_id));
        }
        self.default_tenant_id = Some(tenant_id);
        Ok(self)
    }

    /// Selects a tenant only when it belongs to the authenticated identity.
    pub fn select(&self, tenant_id: &str) -> Result<TenantContext, TenantContextError> {
        validate_tenant_id(tenant_id)?;
        if !self.tenant_ids.iter().any(|allowed| allowed == tenant_id) {
            return Err(TenantContextError::TenantNotInMembership(
                tenant_id.to_string(),
            ));
        }
        TenantContext::try_new(tenant_id)
    }

    /// Returns the authenticated default tenant, or the sole membership when
    /// exactly one tenant is available.
    pub fn default_context(&self) -> Option<TenantContext> {
        let selected = self
            .default_tenant_id
            .as_deref()
            .or_else(|| (self.tenant_ids.len() == 1).then(|| self.tenant_ids[0].as_str()))?;
        TenantContext::try_new(selected).ok()
    }
}

/// Invalid or unauthorized tenant identity state.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum TenantContextError {
    /// Tenant identifiers must use a bounded, unambiguous character set.
    #[error("invalid tenant identifier `{0}`")]
    InvalidTenantId(String),
    /// An authenticated identity must belong to at least one tenant.
    #[error("authenticated tenant membership is empty")]
    EmptyMembership,
    /// The selected tenant is not present in the authenticated identity claims.
    #[error("tenant `{0}` is not present in the authenticated membership")]
    TenantNotInMembership(String),
}

/// Validates the bounded tenant identifier character set.
///
/// Identifiers made only of dots (`.`, `..`, `...`) are rejected because they
/// are relative path components rather than tenant names.
pub(crate) fn validate_tenant_id(tenant_id: &str) -> Result<(), TenantContextError> {
    if tenant_id.is_empty()
        || tenant_id.len() > 128
        || tenant_id.bytes().all(|byte| byte == b'.')
        || !tenant_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(TenantContextError::InvalidTenantId(tenant_id.to_string()));
    }
    Ok(())
}

/// Builds the canonical `tenants:<tenant>:<logical>` backend name shared by
/// tenant-bound cache keys, realtime channels and presence rooms.
///
/// Only the tenant segment is encoded: `%` becomes `%25` and `:` becomes
/// `%3A`. The segment therefore never contains the `:` delimiter, so distinct
/// `(tenant, logical)` pairs always yield distinct names even when the logical
/// name contains `:`. Identifiers without `%` or `:` keep their previous name.
pub(crate) fn tenant_namespaced_name(tenant_id: &str, logical_name: &str) -> String {
    const PREFIX: &str = "tenants:";
    let mut name = String::with_capacity(PREFIX.len() + tenant_id.len() + 1 + logical_name.len());
    name.push_str(PREFIX);
    for c in tenant_id.chars() {
        match c {
            '%' => name.push_str("%25"),
            ':' => name.push_str("%3A"),
            other => name.push(other),
        }
    }
    name.push(':');
    name.push_str(logical_name);
    name
}

fn apply_authenticated_default(req: &mut Request) {
    if req.extensions().get::<TenantContext>().is_some() {
        return;
    }
    let default_context = req
        .extensions()
        .get::<TenantMembership>()
        .and_then(TenantMembership::default_context);
    if let Some(context) = default_context {
        req.extensions_mut().insert(context);
    }
}

/// Propagates tenant context established by trusted authentication middleware.
///
/// Client-controlled tenant headers are deliberately ignored. An upstream
/// authentication layer must insert [`TenantContext`] or [`TenantMembership`]
/// into request extensions after validating claims and membership.
#[cfg_attr(mutants, mutants::skip)]
pub async fn tenant_guard_middleware(mut req: Request, next: Next) -> Response {
    apply_authenticated_default(&mut req);
    next.run(req).await
}

/// Strict guard that rejects requests without authenticated tenant context.
#[cfg_attr(mutants, mutants::skip)]
pub async fn strict_tenant_guard_middleware(mut req: Request, next: Next) -> Response {
    apply_authenticated_default(&mut req);
    if req.extensions().get::<TenantContext>().is_some() {
        next.run(req).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            "Authenticated tenant context is required",
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // TM-TENANT-04
    fn dot_only_tenant_ids_are_rejected() {
        for tenant_id in [".", "..", "..."] {
            assert_eq!(
                TenantContext::try_new(tenant_id),
                Err(TenantContextError::InvalidTenantId(tenant_id.to_string()))
            );
            assert!(TenantMembership::try_new([tenant_id]).is_err());
        }
        for tenant_id in ["acme", "acme.eu", ".acme", "acme.", "acme:prod", "a-b_c"] {
            assert!(TenantContext::try_new(tenant_id).is_ok(), "{tenant_id}");
        }
    }

    #[test]
    // TM-TENANT-04
    fn tenant_namespaced_names_are_injective() {
        assert_eq!(
            tenant_namespaced_name("acme", "leaderboard:x"),
            "tenants:acme:leaderboard:x"
        );
        assert_eq!(tenant_namespaced_name("a", "b:c"), "tenants:a:b:c");
        assert_eq!(tenant_namespaced_name("a:b", "c"), "tenants:a%3Ab:c");
        assert_eq!(tenant_namespaced_name("a%3Ab", "c"), "tenants:a%253Ab:c");
        assert_ne!(
            tenant_namespaced_name("a", "b:c"),
            tenant_namespaced_name("a:b", "c")
        );
        assert_ne!(
            tenant_namespaced_name("a:b", "c"),
            tenant_namespaced_name("a%3Ab", "c")
        );
    }
}
