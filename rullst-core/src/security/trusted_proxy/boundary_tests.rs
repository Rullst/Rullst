//! Exact limits of forwarding-header parsing, rejection logging and the
//! service wrapper.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::parse::{Rejection, resolve_chain, x_forwarded_proto};
use super::*;
use axum::http::{HeaderMap, HeaderValue};
use std::sync::Mutex;

const PROXY: &str = "10.0.0.1:443";

fn proxies() -> TrustedProxyConfig {
    TrustedProxyConfig::new(["10.0.0.0/8"]).expect("valid trusted networks")
}

fn forwarded_proxies() -> TrustedProxyConfig {
    proxies().with_header(ForwardedHeader::Forwarded)
}

fn headers(name: &'static str, value: &str) -> HeaderMap {
    let mut map = HeaderMap::new();
    map.append(name, HeaderValue::from_str(value).expect("header value"));
    map
}

fn chain(
    config: &TrustedProxyConfig,
    name: &'static str,
    value: &str,
) -> Result<IpAddr, Rejection> {
    resolve_chain(config, &headers(name, value)).map(|resolution| resolution.ip)
}

fn ip(text: &str) -> IpAddr {
    text.parse().expect("test address")
}

#[test]
fn up_to_4096_forwarding_bytes_are_examined() {
    // A client entry, then one trusted hop padded with spaces to the limit.
    let header = |len: usize| format!("198.51.100.7,{}10.0.0.1", " ".repeat(len - 21));
    assert_eq!(header(4_096).len(), 4_096);
    assert_eq!(
        chain(&proxies(), "x-forwarded-for", &header(4_096)),
        Ok(ip("198.51.100.7"))
    );
    assert_eq!(
        chain(&proxies(), "x-forwarded-for", &header(4_097)),
        Err(Rejection::Oversized)
    );
}

#[test]
fn x_forwarded_proto_may_fill_32_bytes() {
    let value = |len: usize| format!("https{}", ",".repeat(len - 5));
    assert_eq!(
        x_forwarded_proto(&headers("x-forwarded-proto", &value(32))),
        Some(ForwardedProto::Https)
    );
    assert_eq!(
        x_forwarded_proto(&headers("x-forwarded-proto", &value(33))),
        None
    );
}

#[test]
fn forwarded_nodes_may_fill_64_bytes() {
    let element = |len: usize| format!("for=\"198.51.100.7:_{}\"", "a".repeat(len - 14));
    assert_eq!(
        chain(&forwarded_proxies(), "forwarded", &element(64)),
        Ok(ip("198.51.100.7"))
    );
    assert_eq!(
        chain(&forwarded_proxies(), "forwarded", &element(65)),
        Err(Rejection::Malformed)
    );
}

#[test]
fn forwarded_parameters_need_a_token_name_and_a_clean_quoted_value() {
    let forwarded = forwarded_proxies();
    assert_eq!(
        chain(&forwarded, "forwarded", "by=\"a\tb\";for=198.51.100.7"),
        Ok(ip("198.51.100.7"))
    );
    for malformed in [
        "=x;for=198.51.100.7",
        "b y=x;for=198.51.100.7",
        "by=\"a\"b\"c\";for=198.51.100.7",
        "by=\"a\\b\";for=198.51.100.7",
    ] {
        assert_eq!(
            chain(&forwarded, "forwarded", malformed),
            Err(Rejection::Malformed),
            "{malformed}"
        );
    }
}

#[test]
fn only_forwarded_nodes_may_carry_an_obfuscated_port() {
    assert_eq!(
        chain(&forwarded_proxies(), "forwarded", "for=\"198.51.100.7:_x\""),
        Ok(ip("198.51.100.7"))
    );
    for port in ["_", "ab"] {
        let element = format!("for=\"198.51.100.7:{port}\"");
        assert_eq!(
            chain(&forwarded_proxies(), "forwarded", &element),
            Err(Rejection::Malformed),
            "{port}"
        );
    }
    assert_eq!(
        chain(&proxies(), "x-forwarded-for", "198.51.100.7:_x"),
        Err(Rejection::Malformed)
    );
}

#[test]
fn an_untrusted_scheme_is_ignored_even_without_a_client_address() {
    let resolved = resolve(
        &proxies(),
        PROXY.parse().expect("peer"),
        &headers("x-forwarded-proto", "https"),
    );
    assert!(!resolved.via_trusted_proxy());
    assert_eq!(resolved.forwarded_proto(), None);
    assert!(!ForwardedProto::Http.is_https());
}

#[test]
fn network_prefixes_are_bounded_and_mapped_networks_need_96_bits() {
    assert!(matches!(
        TrustedProxyConfig::new(["10.0.0.0/0024"]),
        Err(TrustedProxyError::InvalidNetwork(_))
    ));
    assert!(matches!(
        TrustedProxyConfig::new(["::ffff:198.51.100.0/80"]),
        Err(TrustedProxyError::HostBitsSet(_))
    ));
    assert_eq!(ForwardedHeader::Forwarded.as_str(), "forwarded");
    assert_eq!(
        ForwardedHeader::XForwardedFor.to_string(),
        "x-forwarded-for"
    );
}

/// Formatted `tracing` output written while a closure runs.
fn captured_logs(run: impl FnOnce()) -> String {
    #[derive(Clone, Default)]
    struct Capture(Arc<Mutex<Vec<u8>>>);
    impl std::io::Write for Capture {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_max_level(tracing::Level::TRACE)
        .with_writer(move || writer.clone())
        .finish();
    tracing::subscriber::with_default(subscriber, run);
    let bytes = capture.0.lock().unwrap();
    String::from_utf8_lossy(&bytes).into_owned()
}

#[test]
fn rejected_chains_are_logged_with_their_reason_and_header() {
    let peer: SocketAddr = PROXY.parse().expect("peer");
    let logs = captured_logs(|| {
        resolve(&proxies(), peer, &headers("x-forwarded-for", "unknown"));
        resolve(&forwarded_proxies(), peer, &HeaderMap::new());
    });
    let lines: Vec<&str> = logs.lines().collect();
    assert_eq!(lines.len(), 2, "{logs}");
    assert!(lines[0].contains("WARN"), "{logs}");
    assert!(lines[0].contains("unidentified"), "{logs}");
    assert!(lines[0].contains("x-forwarded-for"), "{logs}");
    assert!(lines[1].contains("DEBUG"), "{logs}");
    assert!(lines[1].contains("sent no client address"), "{logs}");
    assert!(lines[1].contains("forwarded"), "{logs}");
}

/// A service that is never ready.
#[derive(Clone)]
struct Overloaded;

impl Service<Request<()>> for Overloaded {
    type Response = ();
    type Error = &'static str;
    type Future = std::future::Ready<Result<(), &'static str>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Err("overloaded"))
    }

    fn call(&mut self, _request: Request<()>) -> Self::Future {
        std::future::ready(Ok(()))
    }
}

#[test]
fn the_service_reports_the_readiness_of_its_inner_service() {
    let mut service = TrustedProxyLayer::new(proxies()).layer(Overloaded);
    let waker = std::task::Waker::noop();
    let mut cx = Context::from_waker(waker);
    assert_eq!(service.poll_ready(&mut cx), Poll::Ready(Err("overloaded")));
}
