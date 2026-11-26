//! Security middlewares and utilities for `rullst-core`.
//!
//! Provides CSRF protection, OWASP secure headers, WAF intrusion prevention,
//! zero-alloc PII data masking, explicit trusted-proxy client resolution and
//! the process-local security telemetry
//! ([`SecurityStore`](crate::security::SecurityStore)) that the
//! `rullst-security` layers record into and Studio and Nexus display.

mod baseline;
mod csrf;
mod csrf_mask;
mod csrf_multipart;
mod csrf_random;
mod headers;
mod machine;
mod media_type;
mod pii;
mod secure_cookies;
mod telemetry;
mod tenant_guard;
mod trusted_proxy;
mod waf;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod headers_tests;

#[cfg(test)]
mod header_plan_tests;

#[cfg(test)]
mod csp_nonce_tests;

#[cfg(test)]
mod machine_client_tests;

#[cfg(kani)]
mod kani_proofs;

/// The rest of a middleware stack: axum's [`Next`](axum::middleware::Next),
/// or a closure that runs the layers inside, so several baseline layers run
/// as one middleware.
pub(crate) trait Run: Send {
    fn run(
        self,
        req: axum::extract::Request,
    ) -> impl Future<Output = axum::response::Response> + Send;
}

impl Run for axum::middleware::Next {
    fn run(
        self,
        req: axum::extract::Request,
    ) -> impl Future<Output = axum::response::Response> + Send {
        axum::middleware::Next::run(self, req)
    }
}

impl<F, Fut> Run for F
where
    F: FnOnce(axum::extract::Request) -> Fut + Send,
    Fut: Future<Output = axum::response::Response> + Send,
{
    fn run(
        self,
        req: axum::extract::Request,
    ) -> impl Future<Output = axum::response::Response> + Send {
        self(req)
    }
}

// ─── Public Re-exports ──────────────────────────────────────────────────────

pub use baseline::{
    SecurityBaselineError, apply_security_baseline, apply_security_baseline_with_machine_endpoints,
};
pub use csrf::{CsrfToken, csrf_middleware, generate_csrf_token};
pub use csrf_mask::mask_csrf_token;
pub(crate) use csrf_multipart::{
    MAX_TOKEN_PREFIX_BYTES, MULTIPART_TOKEN_ORDER_MESSAGE, form_data_boundary,
};
pub use headers::{
    CspNonce, DEFAULT_CSP_TEMPLATE, SecurityHeadersApplied, apply_referrer_policy,
    headers_middleware, render_csp_policy,
};
pub use machine::{
    MachineAuthentication, MachineEndpoint, MachineEndpointError, MachineEndpointPolicy,
    MachineRequestVerifier,
};
pub use pii::{mask_pii, pii_masking_middleware};
pub use telemetry::{
    BannedIpRecord, LiveSecurityEvent, SECURITY_EVENT_SCHEMA_VERSION, SecurityStore,
    TelemetrySnapshot, normalize_ip,
};
pub use tenant_guard::{
    TenantContext, TenantContextError, TenantMembership, strict_tenant_guard_middleware,
    tenant_guard_middleware,
};
pub(crate) use tenant_guard::{tenant_namespaced_name, validate_tenant_id};
pub use trusted_proxy::{
    ClientAddr, ForwardedHeader, ForwardedProto, TrustedProxyConfig, TrustedProxyError,
    TrustedProxyLayer, TrustedProxyService,
};
pub use waf::waf_middleware;
pub(crate) use waf::{MultipartFieldInspection, multipart_field_refusal};
