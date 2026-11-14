//! Canonical production middleware ordering contract.
//!
//! The framework owns the transport and baseline-security stages. Session,
//! authentication, tenant resolution and authorization remain application
//! layers because their state and policies are domain-specific.

/// A stage in the canonical inbound production middleware chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ProductionMiddlewareStage {
    /// Accept forwarded identity only through an explicitly trusted proxy policy.
    TrustedProxy,
    /// Reject request bodies above the route/application limit before buffering.
    BodyLimit,
    /// Accept or generate a validated request correlation identifier.
    RequestId,
    /// Start request tracing after correlation identity exists.
    Tracing,
    /// Wrap every inner response with the strict secure-header baseline.
    SecureHeaders,
    /// Apply an explicit origin allowlist; an empty list authorizes no cross-origin caller.
    Cors,
    /// Inspect supported, bounded request data with the configured WAF/RASP policy.
    WafRasp,
    /// Validate browser-originated state-changing requests.
    Csrf,
    /// Resolve and cryptographically validate the application session.
    Session,
    /// Construct the authenticated subject from validated credentials.
    Authentication,
    /// Resolve tenant membership from the authenticated subject.
    Tenant,
    /// Enforce role, permission and object ownership.
    Authorization,
    /// Rate limit by authenticated identity when present, otherwise by direct peer.
    RateLimit,
    /// Dispatch the application handler only after every applicable guard.
    Handler,
}

/// Typed, stable description of the v12 production middleware preset.
///
/// This type documents the outer-to-inner request order. It deliberately does
/// not fabricate generic authentication or authorization: applications mount
/// those domain layers in the slots declared here.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProductionPreset;

impl ProductionPreset {
    /// Canonical outer-to-inner request order for production applications.
    pub const MIDDLEWARE_ORDER: &'static [ProductionMiddlewareStage] = &[
        ProductionMiddlewareStage::TrustedProxy,
        ProductionMiddlewareStage::BodyLimit,
        ProductionMiddlewareStage::RequestId,
        ProductionMiddlewareStage::Tracing,
        ProductionMiddlewareStage::SecureHeaders,
        ProductionMiddlewareStage::Cors,
        ProductionMiddlewareStage::WafRasp,
        ProductionMiddlewareStage::Csrf,
        ProductionMiddlewareStage::Session,
        ProductionMiddlewareStage::Authentication,
        ProductionMiddlewareStage::Tenant,
        ProductionMiddlewareStage::Authorization,
        ProductionMiddlewareStage::RateLimit,
        ProductionMiddlewareStage::Handler,
    ];

    /// Returns the canonical outer-to-inner middleware order.
    pub const fn middleware_order() -> &'static [ProductionMiddlewareStage] {
        Self::MIDDLEWARE_ORDER
    }
}

#[cfg(test)]
mod tests {
    use super::{ProductionMiddlewareStage, ProductionPreset};
    use std::collections::HashSet;

    #[test]
    fn production_order_is_complete_unique_and_ends_at_the_handler() {
        let order = ProductionPreset::middleware_order();
        let unique = order.iter().copied().collect::<HashSet<_>>();

        assert_eq!(order.len(), 14);
        assert_eq!(unique.len(), order.len());
        assert_eq!(
            order.first(),
            Some(&ProductionMiddlewareStage::TrustedProxy)
        );
        assert_eq!(order.last(), Some(&ProductionMiddlewareStage::Handler));
    }

    #[test]
    fn identity_is_validated_before_tenant_authorization_and_rate_limit() {
        let order = ProductionPreset::middleware_order();
        let expected_identity_order = [
            ProductionMiddlewareStage::Session,
            ProductionMiddlewareStage::Authentication,
            ProductionMiddlewareStage::Tenant,
            ProductionMiddlewareStage::Authorization,
            ProductionMiddlewareStage::RateLimit,
        ];
        let observed_identity_order = order
            .iter()
            .copied()
            .filter(|stage| expected_identity_order.contains(stage))
            .collect::<Vec<_>>();

        assert_eq!(observed_identity_order, expected_identity_order);
    }
}
