//! Exact-error, boundary and address-range contracts of [`EgressPolicy`].
use super::{EgressPolicy, EgressPolicyError};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

fn policy() -> EgressPolicy {
    EgressPolicy::strict()
        .with_allowed_hosts(["example.com"])
        .expect("test allowlist")
}

#[test]
fn strict_defaults_bound_redirects_and_response_bytes() {
    let strict = EgressPolicy::strict();
    assert_eq!(strict.max_redirects(), 3);
    assert_eq!(strict.max_response_bytes(), 2_097_152);
    assert_eq!(strict.validate_content_length(Some(2_097_152)), Ok(()));
    assert_eq!(
        strict.validate_content_length(Some(2_097_153)),
        Err(EgressPolicyError::ResponseTooLarge)
    );
    assert_eq!(strict.with_max_redirects(5).max_redirects(), 5);
}

#[test]
fn an_allowlist_holds_at_most_128_hosts() {
    let hosts = |count: usize| (0..count).map(|index| format!("host-{index}.example.com"));
    assert!(
        EgressPolicy::strict()
            .with_allowed_hosts(hosts(128))
            .is_ok()
    );
    assert_eq!(
        EgressPolicy::strict()
            .with_allowed_hosts(hosts(129))
            .map(|_| ()),
        Err(EgressPolicyError::InvalidConfiguration)
    );
}

#[test]
fn any_credential_or_fragment_is_unsafe() {
    for url in [
        "https://user@example.com/",
        "https://:password@example.com/",
        "https://example.com/#fragment",
    ] {
        assert_eq!(
            policy().validate_url(url).map(|_| ()),
            Err(EgressPolicyError::UnsafeUrlComponents),
            "{url}"
        );
    }
}

#[test]
fn local_and_metadata_names_are_blocked_before_the_allowlist() {
    for url in [
        "https://localhost/",
        "https://LOCALHOST./",
        "https://service.localhost/",
        "https://metadata.google.internal/",
        "https://instance-data/",
    ] {
        assert_eq!(
            policy().validate_url(url).map(|_| ()),
            Err(EgressPolicyError::BlockedHost),
            "{url}"
        );
    }
}

fn resolves_publicly(address: IpAddr) -> bool {
    let policy = policy();
    let url = policy
        .validate_url("https://example.com/")
        .expect("allowed URL");
    match policy.validate_resolved_ips(&url, [address]) {
        Ok(_) => true,
        Err(EgressPolicyError::NonPublicAddress) => false,
        Err(other) => panic!("{address}: unexpected {other:?}"),
    }
}

#[test]
fn ipv4_special_purpose_ranges_are_exact() {
    for address in [
        "0.1.2.3",
        "10.0.0.1",
        "127.0.0.1",
        "224.0.0.1",
        "240.0.0.1",
        "100.64.0.1",
        "100.127.255.254",
        "169.254.1.1",
        "172.16.0.1",
        "172.31.255.254",
        "192.168.1.1",
        "198.18.0.1",
        "198.19.255.254",
        "192.0.0.8",
        "192.0.2.1",
        "198.51.100.1",
        "203.0.113.1",
        "255.255.255.255",
    ] {
        let parsed: Ipv4Addr = address.parse().expect("test address");
        assert!(!resolves_publicly(IpAddr::V4(parsed)), "{address}");
    }
    for address in [
        "8.8.8.8",
        "100.0.0.1",
        "100.128.0.1",
        "8.100.1.1",
        "8.254.1.1",
        "169.253.1.1",
        "8.20.1.1",
        "172.15.0.1",
        "172.32.0.1",
        "192.1.1.1",
        "192.1.0.1",
        "8.168.1.1",
        "198.1.1.1",
        "198.20.0.1",
        "8.18.1.1",
        "8.0.0.1",
        "8.8.0.8",
        "192.0.1.1",
        "223.255.255.254",
    ] {
        let parsed: Ipv4Addr = address.parse().expect("test address");
        assert!(resolves_publicly(IpAddr::V4(parsed)), "{address}");
    }
}

#[test]
fn ipv6_documentation_and_ietf_ranges_are_exact() {
    for address in ["2001:4860:4860::8888", "2606:100::1", "2606:db8::1"] {
        let parsed: Ipv6Addr = address.parse().expect("test address");
        assert!(resolves_publicly(IpAddr::V6(parsed)), "{address}");
    }
    for address in ["2001:1ff::1", "2001:db8::1"] {
        let parsed: Ipv6Addr = address.parse().expect("test address");
        assert!(!resolves_publicly(IpAddr::V6(parsed)), "{address}");
    }
}
