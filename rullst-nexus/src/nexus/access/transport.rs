//! Transport-security evidence accepted by the Nexus Basic Auth guard.

use axum::extract::Request;
use rullst_core::security::{ClientAddr, ForwardedProto};

use super::NexusVerifiedTls;

/// Basic credentials require explicit TLS evidence: the application-inserted
/// [`NexusVerifiedTls`] marker, or an HTTPS scheme that Core's
/// `TrustedProxyLayer` accepted from a trusted proxy peer with
/// `trust_forwarded_proto` enabled. `ClientAddr` has no public constructor, so
/// a forwarding header from an arbitrary client can never produce it.
pub(super) fn has_verified_tls(request: &Request) -> bool {
    let extensions = request.extensions();
    extensions.get::<NexusVerifiedTls>().is_some()
        || extensions
            .get::<ClientAddr>()
            .and_then(ClientAddr::forwarded_proto)
            .is_some_and(ForwardedProto::is_https)
}
