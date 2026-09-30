#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::parse::{Rejection, resolve_chain};
use super::*;
use axum::http::{HeaderMap, HeaderValue};

const PROXY: &str = "10.0.0.1:443";

fn config(networks: &[&str]) -> TrustedProxyConfig {
    TrustedProxyConfig::new(networks).expect("valid trusted networks")
}

fn forwarded_config(networks: &[&str]) -> TrustedProxyConfig {
    config(networks)
        .with_header(ForwardedHeader::Forwarded)
        .trust_forwarded_proto(true)
}

fn headers(pairs: &[(&'static str, &str)]) -> HeaderMap {
    let mut map = HeaderMap::new();
    for (name, value) in pairs {
        map.append(*name, HeaderValue::from_str(value).expect("header value"));
    }
    map
}

fn ip(text: &str) -> IpAddr {
    text.parse().expect("test address")
}

fn client(config: &TrustedProxyConfig, peer: &str, pairs: &[(&'static str, &str)]) -> ClientAddr {
    resolve(config, peer.parse().expect("test peer"), &headers(pairs))
}

fn assert_kept(config: &TrustedProxyConfig, pairs: &[(&'static str, &str)], reason: Rejection) {
    let resolved = client(config, PROXY, pairs);
    assert_eq!(resolved.ip(), ip("10.0.0.1"), "{pairs:?}");
    assert!(!resolved.via_trusted_proxy(), "{pairs:?}");
    assert_eq!(
        resolve_chain(config, &headers(pairs)),
        Err(reason),
        "{pairs:?}"
    );
}

#[test]
fn config_accepts_hosts_and_networks_and_normalizes_mapped_ipv6() {
    let proxies = config(&[
        "10.0.0.0/8",
        "192.0.2.7",
        "2001:db8::/32",
        "::1",
        "::ffff:198.51.100.0/120",
    ]);
    assert!(proxies.is_enabled());
    assert!(proxies.is_trusted(ip("10.255.0.1")));
    assert!(!proxies.is_trusted(ip("11.0.0.1")));
    assert!(proxies.is_trusted(ip("192.0.2.7")));
    assert!(!proxies.is_trusted(ip("192.0.2.8")));
    assert!(proxies.is_trusted(ip("2001:db8:1::1")));
    assert!(!proxies.is_trusted(ip("2001:db9::1")));
    assert!(proxies.is_trusted(ip("::1")));
    assert!(proxies.is_trusted(ip("198.51.100.200")));
    assert!(proxies.is_trusted(ip("::ffff:10.1.1.1")));
    let debug = format!("{proxies:?}");
    assert!(debug.contains("\"10.0.0.0/8\""), "{debug}");
    assert!(debug.contains("\"198.51.100.0/24\""), "{debug}");
    assert_eq!(proxies.header(), ForwardedHeader::XForwardedFor);
    assert!(!proxies.trusts_forwarded_proto());
}

#[test]
fn config_rejects_invalid_unbounded_duplicate_and_excess_networks() {
    let error = |entries: &[&str]| TrustedProxyConfig::new(entries).expect_err("invalid");
    for entry in [
        "",
        "10.0.0.0/33",
        "10.0.0.0/",
        "10.0.0.0/+8",
        "010.0.0.1",
        "fe80::1%eth0",
        "::/129",
        "proxy.internal",
    ] {
        assert_eq!(
            error(&[entry]),
            TrustedProxyError::InvalidNetwork(entry.to_string())
        );
    }
    assert_eq!(
        error(&["10.0.0.1/8"]),
        TrustedProxyError::HostBitsSet("10.0.0.1/8".into())
    );
    for entry in ["0.0.0.0/0", "::/0", "::ffff:0.0.0.0/96"] {
        assert_eq!(
            error(&[entry]),
            TrustedProxyError::TrustsEveryAddress(entry.into())
        );
    }
    assert_eq!(
        error(&["10.0.0.0/8", "::ffff:10.0.0.0/104"]),
        TrustedProxyError::DuplicateNetwork("::ffff:10.0.0.0/104".into())
    );
    let many = (0..=TrustedProxyConfig::MAX_NETWORKS)
        .map(|index| format!("10.0.{index}.0/24"))
        .collect::<Vec<_>>();
    assert_eq!(
        TrustedProxyConfig::new(&many),
        Err(TrustedProxyError::TooManyNetworks { maximum: 64 })
    );
    assert!(TrustedProxyConfig::new(&many[..64]).is_ok());

    // Echoed values are bounded and cannot inject log lines.
    let long = format!("bad\nentry{}", "x".repeat(200));
    let TrustedProxyError::InvalidNetwork(echo) = error(&[long.as_str()]) else {
        panic!("invalid network expected");
    };
    assert!(echo.starts_with("bad?entry"));
    assert_eq!(echo.chars().count(), 64);

    let disabled = TrustedProxyConfig::new(Vec::<String>::new()).unwrap();
    assert!(!disabled.is_enabled());
    assert_eq!(disabled, TrustedProxyConfig::default());
    assert_eq!("Forwarded".parse(), Ok(ForwardedHeader::Forwarded));
    assert_eq!(
        "X-Forwarded-For".parse(),
        Ok(ForwardedHeader::XForwardedFor)
    );
    assert_eq!(
        "x-real-ip".parse::<ForwardedHeader>(),
        Err(TrustedProxyError::InvalidHeader("x-real-ip".into()))
    );
}

#[test]
fn rullst_toml_settings_are_optional_and_validated() {
    let parsed = crate::config::RullstConfig::from_toml(
        "[security]\ntrusted_proxies = [\"10.0.0.0/8\", \"::1\"]\n\
         trusted_proxy_header = \"forwarded\"\ntrust_forwarded_proto = true\n",
    )
    .unwrap();
    parsed.validate().unwrap();
    let proxies = TrustedProxyConfig::from_security_config(&parsed.security).unwrap();
    assert!(proxies.is_trusted(ip("::1")));
    assert_eq!(proxies.header(), ForwardedHeader::Forwarded);
    assert!(proxies.trusts_forwarded_proto());

    let defaults = crate::config::RullstConfig::from_toml("").unwrap();
    assert_eq!(
        TrustedProxyConfig::from_security_config(&defaults.security),
        Ok(TrustedProxyConfig::default())
    );
    for invalid in [
        "[security]\ntrusted_proxies = [\"10.0.0.1/8\"]\n",
        "[security]\ntrusted_proxies = [\"0.0.0.0/0\"]\n",
        "[security]\ntrusted_proxy_header = \"x-real-ip\"\n",
    ] {
        let parsed = crate::config::RullstConfig::from_toml(invalid).unwrap();
        assert!(matches!(
            parsed.validate(),
            Err(crate::config::ConfigError::InvalidSecurityConfiguration(_))
        ));
    }
}

#[test]
fn untrusted_peer_cannot_choose_its_identity_through_forwarding_headers() {
    // TM-CORE-03: a direct client forging every forwarding header keeps its socket identity.
    let forged = [
        ("x-forwarded-for", "198.51.100.1"),
        ("forwarded", "for=198.51.100.2;proto=https"),
        ("x-forwarded-proto", "https"),
    ];
    for proxies in [
        config(&["10.0.0.0/8"]).trust_forwarded_proto(true),
        forwarded_config(&["10.0.0.0/8"]),
    ] {
        let resolved = client(&proxies, "203.0.113.9:5000", &forged);
        assert_eq!(resolved.ip(), ip("203.0.113.9"));
        assert_eq!(resolved.peer(), "203.0.113.9:5000".parse().unwrap());
        assert!(!resolved.via_trusted_proxy());
        assert_eq!(resolved.forwarded_proto(), None);
    }
    // A disabled policy never reads headers, even from a would-be proxy.
    let resolved = client(&TrustedProxyConfig::default(), PROXY, &forged);
    assert_eq!(resolved.ip(), ip("10.0.0.1"));
    assert!(!resolved.via_trusted_proxy());
}

#[test]
fn trusted_peer_resolves_single_and_multi_hop_chains() {
    let proxies = config(&["10.0.0.0/8"]);
    for (chain, expected) in [
        (vec!["203.0.113.5"], "203.0.113.5"),
        // Spoofed left-most entry, real client, then another trusted proxy.
        (vec!["198.51.100.77, 203.0.113.5, 10.0.0.2"], "203.0.113.5"),
        // Every hop is trusted: the left-most is the client.
        (vec!["10.0.0.3, 10.0.0.2"], "10.0.0.3"),
        // Header lines form one list in their received order.
        (
            vec!["198.51.100.77", "203.0.113.5, 10.0.0.2"],
            "203.0.113.5",
        ),
        (vec!["203.0.113.5", "10.0.0.2"], "203.0.113.5"),
        (vec![" 203.0.113.5,, 10.0.0.2 , "], "203.0.113.5"),
    ] {
        let pairs = chain
            .iter()
            .map(|value| ("x-forwarded-for", *value))
            .collect::<Vec<_>>();
        let resolved = client(&proxies, PROXY, &pairs);
        assert_eq!(resolved.ip(), ip(expected), "{chain:?}");
        assert!(resolved.via_trusted_proxy());
        assert_eq!(resolved.peer(), PROXY.parse().unwrap());
    }
}

#[test]
fn addresses_accept_ports_brackets_ipv6_and_mapped_forms() {
    let proxies = config(&["10.0.0.0/8", "::1"]);
    for (value, expected) in [
        ("203.0.113.5:4711", "203.0.113.5"),
        ("[2001:db8::7]:443", "2001:db8::7"),
        ("[2001:db8::7]", "2001:db8::7"),
        ("2001:db8::7", "2001:db8::7"),
        ("::ffff:203.0.113.5", "203.0.113.5"),
        ("[::ffff:203.0.113.5]:80", "203.0.113.5"),
    ] {
        let resolved = client(&proxies, PROXY, &[("x-forwarded-for", value)]);
        assert_eq!(resolved.ip(), ip(expected), "{value}");
    }
    for peer in ["[::1]:8080", "[::ffff:10.0.0.1]:443"] {
        let resolved = client(&proxies, peer, &[("x-forwarded-for", "203.0.113.5")]);
        assert_eq!(resolved.ip(), ip("203.0.113.5"), "{peer}");
    }
}

#[test]
fn unusable_chains_from_a_trusted_peer_keep_the_peer() {
    let proxies = config(&["10.0.0.0/8"]);
    assert_kept(&proxies, &[], Rejection::Missing);
    assert_kept(&proxies, &[("x-forwarded-for", " , ")], Rejection::Missing);
    assert_kept(
        &proxies,
        &[("x-forwarded-for", "unknown")],
        Rejection::Unidentified,
    );
    for malformed in [
        "not-an-ip",
        "203.0.113.5:99999",
        "203.0.113.5:",
        "\"203.0.113.5\"",
        "fe80::1%eth0",
        "[2001:db8::1",
        "[2001:db8::1]x",
        "203.0.113.5 198.51.100.1",
    ] {
        assert_kept(
            &proxies,
            &[("x-forwarded-for", malformed)],
            Rejection::Malformed,
        );
    }
    let mut non_ascii = HeaderMap::new();
    non_ascii.insert(
        "x-forwarded-for",
        HeaderValue::from_bytes(b"203.0.113.5\xff").unwrap(),
    );
    assert_eq!(
        resolve_chain(&proxies, &non_ascii),
        Err(Rejection::Malformed)
    );

    let oversized = "1".repeat(TrustedProxyConfig::MAX_HEADER_BYTES + 1);
    assert_kept(
        &proxies,
        &[("x-forwarded-for", &oversized)],
        Rejection::Oversized,
    );

    let chain = |hops: usize| vec!["10.0.0.9"; hops].join(", ");
    assert_kept(
        &proxies,
        &[("x-forwarded-for", &chain(TrustedProxyConfig::MAX_HOPS + 1))],
        Rejection::TooManyHops,
    );
    let at_limit = client(
        &proxies,
        PROXY,
        &[("x-forwarded-for", &chain(TrustedProxyConfig::MAX_HOPS))],
    );
    assert_eq!(at_limit.ip(), ip("10.0.0.9"));
}

#[test]
fn client_supplied_text_left_of_its_own_entry_is_never_parsed() {
    let proxies = config(&["10.0.0.0/8"]);
    let padding = format!("{}, unknown, not-an-ip", "9".repeat(8 * 1024));
    let padded = format!("{padding}, 203.0.113.5, 10.0.0.2");
    let hops = format!("{}203.0.113.5", "198.51.100.1, ".repeat(100));
    for value in [padded.as_str(), hops.as_str()] {
        let resolved = client(&proxies, PROXY, &[("x-forwarded-for", value)]);
        assert_eq!(resolved.ip(), ip("203.0.113.5"));
        assert!(resolved.via_trusted_proxy());
    }
}

#[test]
fn forwarded_mode_parses_rfc_7239_elements() {
    let proxies = forwarded_config(&["10.0.0.0/8"]);
    for (value, expected, proto) in [
        ("for=203.0.113.5", "203.0.113.5", None),
        (
            "For=\"[2001:db8:cafe::17]:4711\"",
            "2001:db8:cafe::17",
            None,
        ),
        (
            "for=\"203.0.113.5:4711\";proto=https;by=10.0.0.1",
            "203.0.113.5",
            Some(ForwardedProto::Https),
        ),
        (
            "for=198.51.100.77, for=203.0.113.5;proto=HTTPS, for=10.0.0.2;proto=http",
            "203.0.113.5",
            Some(ForwardedProto::Https),
        ),
        ("for=\"[2001:db8::1]:_abc\"", "2001:db8::1", None),
        (
            "for=203.0.113.5;host=\"example.com;x\"",
            "203.0.113.5",
            None,
        ),
        (
            " for=203.0.113.5 ; proto=http ",
            "203.0.113.5",
            Some(ForwardedProto::Http),
        ),
    ] {
        let resolved = client(&proxies, PROXY, &[("forwarded", value)]);
        assert_eq!(resolved.ip(), ip(expected), "{value}");
        assert!(resolved.via_trusted_proxy(), "{value}");
        assert_eq!(resolved.forwarded_proto(), proto, "{value}");
    }
    // Exactly one header per policy: X-Forwarded-* is ignored in this mode.
    assert_kept(
        &proxies,
        &[
            ("x-forwarded-for", "203.0.113.5"),
            ("x-forwarded-proto", "https"),
        ],
        Rejection::Missing,
    );
    assert_eq!(
        client(&proxies, PROXY, &[("x-forwarded-proto", "https")]).forwarded_proto(),
        None
    );
    let xff = config(&["10.0.0.0/8"]);
    assert_kept(
        &xff,
        &[("forwarded", "for=203.0.113.5")],
        Rejection::Missing,
    );
}

#[test]
fn forwarded_mode_rejects_unidentified_and_malformed_elements() {
    let proxies = forwarded_config(&["10.0.0.0/8"]);
    for unidentified in ["for=unknown", "for=_hidden", "for=\"_gazonk\""] {
        assert_kept(
            &proxies,
            &[("forwarded", unidentified)],
            Rejection::Unidentified,
        );
    }
    for malformed in [
        "for=[2001:db8::1]",
        "for=\"2001:db8::1\"",
        "for=203.0.113.5:80",
        "proto=https",
        "for=203.0.113.5;for=198.51.100.1",
        "for=\"203.0.113.5",
        "for=\"203.0.113.5\\\"\"",
        "for",
        "=203.0.113.5",
        "for=",
        "for=203.0.113.5;proto=https;proto=http",
        // A quoted comma cannot inject a separate element.
        "for=203.0.113.5;host=\"a,for=198.51.100.9\"",
    ] {
        assert_kept(&proxies, &[("forwarded", malformed)], Rejection::Malformed);
    }
}

fn peer_proto(
    proxies: &TrustedProxyConfig,
    pairs: &[(&'static str, &str)],
) -> Option<ForwardedProto> {
    client(proxies, PROXY, pairs).forwarded_proto()
}

fn with_client(proto: &str) -> Vec<(&'static str, &str)> {
    vec![
        ("x-forwarded-for", "203.0.113.5"),
        ("x-forwarded-proto", proto),
    ]
}

#[test]
fn forwarded_proto_requires_explicit_trust_and_one_value() {
    let untrusting = config(&["10.0.0.0/8"]);
    let trusting = config(&["10.0.0.0/8"]).trust_forwarded_proto(true);

    assert_eq!(peer_proto(&untrusting, &with_client("https")), None);
    assert_eq!(
        peer_proto(&trusting, &with_client("https")),
        Some(ForwardedProto::Https)
    );
    assert_eq!(
        peer_proto(&trusting, &with_client("HTTP")),
        Some(ForwardedProto::Http)
    );
    let too_long = "https ".repeat(10);
    for ambiguous in ["https, http", "wss", "", "https,https", too_long.as_str()] {
        assert_eq!(
            peer_proto(&trusting, &with_client(ambiguous)),
            None,
            "{ambiguous}"
        );
    }
    assert_eq!(
        peer_proto(
            &trusting,
            &[
                ("x-forwarded-proto", "https"),
                ("x-forwarded-proto", "https")
            ]
        ),
        None
    );
    // The scheme is a statement of the trusted peer, independent of the address list.
    let health_check = client(&trusting, PROXY, &[("x-forwarded-proto", "https")]);
    assert!(!health_check.via_trusted_proxy());
    assert_eq!(health_check.forwarded_proto(), Some(ForwardedProto::Https));
    assert!(ForwardedProto::Https.is_https());
    assert_eq!(ForwardedProto::Http.as_str(), "http");
}
