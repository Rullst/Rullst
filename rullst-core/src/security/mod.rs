//! Security middlewares and utilities for `rullst-core`.
//!
//! Provides CSRF protection, OWASP secure headers, WAF intrusion prevention,
//! and zero-alloc PII data masking.

mod csrf;
mod headers;
mod pii;
mod waf;

#[cfg(test)]
mod tests;

#[cfg(kani)]
mod kani_proofs;

// ─── Public Re-exports ──────────────────────────────────────────────────────

pub use csrf::{csrf_middleware, generate_csrf_token};
pub use headers::headers_middleware;
pub use pii::{mask_pii, pii_masking_middleware};
pub use waf::waf_middleware;
