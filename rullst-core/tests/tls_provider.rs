//! `Server` start-up installs aws-lc-rs as the rustls process default.
//!
//! Process-wide state: this binary holds a single test, so no other test can
//! install a rustls provider first. The features listed below are the ones
//! that build rustls clients in Core.

#![cfg(any(feature = "orm", feature = "storage-s3", feature = "telemetry"))]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use rullst_core::{Server, routes};
use rustls::crypto::{CryptoProvider, aws_lc_rs};
use std::sync::Arc;
use std::time::Duration;

fn kx_names(provider: &CryptoProvider) -> Vec<rustls::NamedGroup> {
    provider
        .kx_groups
        .iter()
        .map(|group| group.name())
        .collect()
}

async fn start() {
    // Bind an ephemeral port and stop at once.
    tokio::time::timeout(
        Duration::from_secs(30),
        Server::new(routes![get("/" => || async { "ok" })]).run_with_shutdown(0, async {}),
    )
    .await
    .expect("startup finishes within the deadline")
    .expect("the server starts");
}

#[tokio::test]
async fn server_startup_installs_aws_lc_rs_unless_a_provider_is_installed() {
    assert!(CryptoProvider::get_default().is_none());

    start().await;
    let installed = CryptoProvider::get_default().expect("start-up installs a provider");
    // aws-lc-rs, not ring: only aws-lc-rs offers the post-quantum hybrid group.
    assert_eq!(
        kx_names(installed),
        kx_names(&aws_lc_rs::default_provider())
    );
    assert!(kx_names(installed).contains(&rustls::NamedGroup::X25519MLKEM768));

    // An installed provider, such as an application's own, stays in force.
    start().await;
    let current = CryptoProvider::get_default().expect("provider still installed");
    assert!(Arc::ptr_eq(installed, current));
}
