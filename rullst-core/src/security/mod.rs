//! Security middlewares and utilities for `rullst-core`.
//!
//! Provides CSRF protection, OWASP secure headers, WAF intrusion prevention,
//! and zero-alloc PII data masking.

mod baseline;
mod csrf;
mod headers;
mod machine;
mod pii;
mod tenant_guard;
mod waf;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod machine_client_tests;

#[cfg(kani)]
mod kani_proofs;

// ─── Public Re-exports ──────────────────────────────────────────────────────

pub use baseline::{
    SecurityBaselineError, apply_security_baseline, apply_security_baseline_with_machine_endpoints,
};
pub use csrf::{CsrfToken, csrf_middleware, generate_csrf_token};
pub use headers::{
    CspNonce, DEFAULT_CSP_TEMPLATE, apply_referrer_policy, headers_middleware, render_csp_policy,
};
pub use machine::{
    MachineAuthentication, MachineEndpoint, MachineEndpointError, MachineEndpointPolicy,
    MachineRequestVerifier,
};
pub use pii::{mask_pii, pii_masking_middleware};
pub use tenant_guard::{
    TenantContext, TenantContextError, TenantMembership, strict_tenant_guard_middleware,
    tenant_guard_middleware,
};
pub use waf::waf_middleware;
