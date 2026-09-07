use axum::{Extension, Router, extract::ConnectInfo};
use base64::Engine;
use rullst_nexus::{Nexus, NexusVerifiedTls};
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

pub const TEST_ADMIN_USERNAME: &str = "nexus-test-admin";

fn test_admin_password() -> &'static str {
    static PASSWORD: OnceLock<String> = OnceLock::new();

    PASSWORD.get_or_init(|| {
        format!(
            "nexus_integration_test_{:016x}_{:08x}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            std::process::id()
        )
    })
}

/// Builds a profile-independent test router through the production Basic Auth
/// and verified-transport boundary.
pub fn authenticated_test_router(nexus: Nexus) -> Router {
    nexus
        .with_auth(TEST_ADMIN_USERNAME, test_admin_password())
        .try_build()
        .expect("valid authenticated Nexus test configuration")
        .layer(Extension(NexusVerifiedTls::from_trusted_tls_termination()))
        .layer(Extension(ConnectInfo(SocketAddr::from((
            Ipv4Addr::LOCALHOST,
            3000,
        )))))
}

/// An authenticated browser request to the local test CMS. Keep these HTTP
/// inputs explicit in fixtures rather than bypassing the production access
/// layer.
pub fn local_request() -> axum::http::request::Builder {
    let credentials = base64::engine::general_purpose::STANDARD
        .encode(format!("{TEST_ADMIN_USERNAME}:{}", test_admin_password()));
    axum::http::Request::builder()
        .header("host", "localhost:3000")
        .header("origin", "http://localhost:3000")
        .header("authorization", format!("Basic {credentials}"))
}
