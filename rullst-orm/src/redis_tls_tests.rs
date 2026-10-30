use super::install_provider_for;
use redis::IntoConnectionInfo;
use rustls::crypto::{CryptoProvider, aws_lc_rs};

fn kx_names(provider: &CryptoProvider) -> Vec<rustls::NamedGroup> {
    provider
        .kx_groups
        .iter()
        .map(|group| group.name())
        .collect()
}

/// No other test in this binary installs a rustls provider; a plaintext
/// endpoint must not install one either.
#[test]
fn only_tls_endpoints_install_aws_lc_rs_as_the_process_default() {
    let plaintext = "redis://127.0.0.1:6379".into_connection_info().unwrap();
    install_provider_for(&plaintext);
    assert!(CryptoProvider::get_default().is_none());

    let tls = "rediss://127.0.0.1:6380".into_connection_info().unwrap();
    install_provider_for(&tls);
    let installed = CryptoProvider::get_default().expect("installed provider");
    // aws-lc-rs offers the post-quantum hybrid group; ring has no ML-KEM.
    assert_eq!(
        kx_names(installed),
        kx_names(&aws_lc_rs::default_provider())
    );
    assert!(kx_names(installed).contains(&rustls::NamedGroup::X25519MLKEM768));

    // A second TLS connection keeps the installed provider.
    install_provider_for(&tls);
    assert!(std::sync::Arc::ptr_eq(
        installed,
        CryptoProvider::get_default().expect("still installed")
    ));
}
