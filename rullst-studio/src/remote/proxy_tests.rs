//! Proxy routing: settings, the proxy variables by target scheme, `NO_PROXY`
//! rules and requests through a local proxy stub (`TM-STUDIO-3`,
//! `REMOTE-13`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::client::{FetchError, FetchLimits, MonitorClient};
use super::projects::{Project, ProjectStore};
use super::proxy::{ProxyMode, ProxyPlan, ProxyRoute, ProxySettings, Unusable, parse_proxy_url};
use super::proxy_env_tests::{proxies, through};
use super::server_test::{TOKEN, TempDir};
use super::webhook::{WebhookClient, WebhookPayload};
use reqwest::Url;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub(super) fn url(value: &str) -> Url {
    Url::parse(value).unwrap()
}

fn route(no_proxy: &str) -> ProxyRoute {
    ProxyRoute::new(url("http://proxy.internal:3128"), no_proxy)
}

pub(super) fn proxied(route: &ProxyRoute, target: &str) -> bool {
    route.proxy_for(&url(target)).is_some()
}

#[test]
fn proxy_urls_are_http_or_https_with_a_host_only() {
    for (value, credentials, valid) in [
        ("http://proxy.internal:3128", false, true),
        (" https://proxy.internal/ ", false, true),
        ("http://user:secret@proxy.internal:3128", true, true),
        ("http://user:secret@proxy.internal:3128", false, false),
        ("http://user@proxy.internal", false, false),
        ("socks5://proxy.internal:1080", false, false),
        ("socks5h://user:secret@proxy.internal", true, false),
        ("http://proxy.internal/path", false, false),
        ("http://proxy.internal/?q=1", false, false),
        ("http://proxy.internal/#f", false, false),
        ("ftp://proxy.internal", false, false),
        ("", false, false),
    ] {
        assert_eq!(
            parse_proxy_url(value, credentials).is_some(),
            valid,
            "{value} {credentials}"
        );
    }
    let longest = format!("http://{}.example", "a".repeat(2_048 - 15));
    assert_eq!(longest.len(), 2_048);
    assert!(parse_proxy_url(&longest, false).is_some());
    assert!(parse_proxy_url(&format!("{longest}x"), false).is_none());
    // Without a scheme, `http://` is meant, as curl assumes.
    assert_eq!(
        parse_proxy_url(" proxy.internal:3128 ", false),
        Some(url("http://proxy.internal:3128"))
    );
    assert_eq!(
        parse_proxy_url("user:secret@proxy.internal", true),
        Some(url("http://user:secret@proxy.internal"))
    );
    assert_eq!(parse_proxy_url("user:secret@proxy.internal", false), None);
}

#[test]
fn loopback_is_always_direct() {
    let route = route("");
    assert_eq!(
        route.proxy_for(&url("https://shop.example/_rullst/monitor/v2")),
        Some(url("http://proxy.internal:3128"))
    );
    assert_eq!(
        route.proxy_for(&url("http://shop.example/")),
        Some(url("http://proxy.internal:3128"))
    );
    for direct in [
        "http://localhost:8080/",
        "http://127.0.0.1:9000/",
        "https://localhost:8443/",
        "https://LOCALHOST/",
        "https://app.localhost/",
        "https://127.0.0.1:9000/",
        "https://127.8.9.10/",
        "https://[::1]:9000/",
        "https://[::ffff:127.0.0.1]/",
        "ftp://shop.example/",
    ] {
        assert!(!proxied(&route, direct), "{direct}");
    }
    assert!(proxied(&route, "https://localhost.example/"));
    assert!(proxied(&route, "https://10.0.0.1/"));
}

#[test]
fn no_proxy_entries_match_domains_addresses_and_ranges() {
    let route =
        route(".internal, Example.COM\t10.0.0.0/8 192.168.1.7,[2001:db8::]/32 ,, 2001:470::5");
    for direct in [
        "https://ci.internal/",
        "https://internal/",
        "https://example.com/",
        "https://api.EXAMPLE.com/",
        "https://10.200.3.4/",
        "https://192.168.1.7/",
        "https://[2001:db8:ffff::1]/",
        "https://[2001:470::5]/",
    ] {
        assert!(!proxied(&route, direct), "{direct}");
    }
    for through in [
        "https://notexample.com/",
        "https://example.com.evil/",
        "https://internal.evil/",
        "https://11.0.0.1/",
        "https://192.168.1.8/",
        "https://[2001:db9::1]/",
        "https://[2001:470::6]/",
    ] {
        assert!(proxied(&route, through), "{through}");
    }
}

#[test]
fn prefix_lengths_are_exact_and_bounded() {
    for (no_proxy, target, direct) in [
        ("10.1.0.0/16", "https://10.1.255.255/", true),
        ("10.1.0.0/16", "https://10.2.0.0/", false),
        ("10.1.2.3/32", "https://10.1.2.3/", true),
        ("10.1.2.3/32", "https://10.1.2.4/", false),
        ("10.1.2.3/40", "https://10.1.2.3/", true),
        ("10.1.2.3/40", "https://10.1.2.4/", false),
        ("0.0.0.0/0", "https://203.0.113.9/", true),
        ("0.0.0.0/0", "https://[2001:db8::1]/", false),
        ("::/0", "https://[2001:db8::1]/", true),
        ("::/0", "https://203.0.113.9/", false),
        ("10.0.0.0/x", "https://10.0.0.1/", false),
        ("*", "https://anything.example/", true),
        (".", "https://shop.example./", false),
    ] {
        assert_eq!(
            !proxied(&route(no_proxy), target),
            direct,
            "{no_proxy} {target}"
        );
    }
}

#[test]
fn explicit_and_direct_modes_ignore_the_environment() {
    let anything = |_: &str| Some("http://env-proxy:3128".to_string());
    assert_eq!(
        ProxyPlan::resolve(&ProxySettings::default(), anything),
        ProxyPlan::Direct
    );
    let explicit = ProxySettings {
        mode: ProxyMode::Explicit,
        url: Some("http://proxy.internal:3128".to_string()),
        no_proxy: "skip.example".to_string(),
    };
    let plan = ProxyPlan::resolve(&explicit, anything);
    assert_eq!(
        proxies(&plan),
        (
            Some(url("http://proxy.internal:3128")),
            Some(url("http://proxy.internal:3128"))
        )
    );
    assert!(!proxied(through(&plan), "https://skip.example/"));
    assert!(explicit.valid());
    for (invalid, unusable) in [
        (
            ProxySettings {
                url: None,
                ..explicit.clone()
            },
            Unusable::Malformed("The saved proxy URL"),
        ),
        (
            ProxySettings {
                url: Some("http://user:secret@proxy.internal".to_string()),
                ..explicit.clone()
            },
            Unusable::Malformed("The saved proxy URL"),
        ),
        (
            ProxySettings {
                url: Some("socks5://proxy.internal".to_string()),
                ..explicit.clone()
            },
            Unusable::Socks("The saved proxy URL"),
        ),
        (
            ProxySettings {
                no_proxy: "a".repeat(4_097),
                ..explicit.clone()
            },
            Unusable::LongExceptions("The saved exception list"),
        ),
    ] {
        assert!(!invalid.valid(), "{invalid:?}");
        assert_eq!(
            ProxyPlan::resolve(&invalid, anything),
            ProxyPlan::Invalid(unusable)
        );
    }
    let environment = ProxySettings {
        mode: ProxyMode::Environment,
        url: None,
        no_proxy: "a".repeat(4_096),
    };
    assert!(environment.valid());
    assert!(ProxySettings::default().valid());
    let newer = ProxySettings {
        mode: ProxyMode::Newer("pac".to_string()),
        ..ProxySettings::default()
    };
    assert!(!newer.valid());
    assert_eq!(
        ProxyPlan::resolve(&newer, anything),
        ProxyPlan::Invalid(Unusable::NewerMode)
    );
}

#[test]
fn the_settings_page_describes_each_plan_without_credentials() {
    assert_eq!(ProxyPlan::Direct.describe(), "Connections are direct.");
    for (proxy, shown) in [
        (
            "http://user:secret@proxy.internal:3128",
            "http://proxy.internal:3128",
        ),
        ("https://proxy.internal", "https://proxy.internal"),
    ] {
        let through = ProxyPlan::Through(ProxyRoute::new(url(proxy), ""));
        assert_eq!(
            through.describe(),
            format!(
                "https:// connections go through {shown} as a CONNECT tunnel, except to loopback and the listed exceptions."
            )
        );
    }
    for (unusable, text) in [
        (
            Unusable::Malformed("HTTPS_PROXY"),
            "HTTPS_PROXY is not a usable proxy URL (http:// or https:// with a host and port only), so Studio connects to nothing: fix it or choose another option.",
        ),
        (
            Unusable::Socks("all_proxy"),
            "all_proxy names a SOCKS proxy, which Studio does not support, so it connects to nothing: use an http:// or https:// proxy, or choose another option.",
        ),
        (
            Unusable::LongExceptions("NO_PROXY"),
            "NO_PROXY is longer than 4096 characters, so Studio connects to nothing: shorten it or choose another option.",
        ),
        (
            Unusable::NewerMode,
            "A newer Studio saved a proxy choice this Studio does not know, so it connects to nothing: choose one here and save.",
        ),
    ] {
        assert_eq!(ProxyPlan::Invalid(unusable).describe(), text);
    }
}

/// A proxy that records each request head it receives and refuses it.
pub(super) struct ProxyStub {
    pub(super) url: String,
    pub(super) heads: Arc<Mutex<Vec<String>>>,
}

impl ProxyStub {
    pub(super) async fn start() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let heads = Arc::new(Mutex::new(Vec::new()));
        let seen = heads.clone();
        tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let seen = seen.clone();
                tokio::spawn(async move {
                    let mut head = Vec::new();
                    let mut buffer = [0_u8; 1024];
                    while !head.ends_with(b"\r\n\r\n") && head.len() < 16 * 1024 {
                        match stream.read(&mut buffer).await {
                            Ok(0) | Err(_) => break,
                            Ok(read) => head.extend_from_slice(&buffer[..read]),
                        }
                    }
                    seen.lock()
                        .unwrap()
                        .push(String::from_utf8_lossy(&head).into_owned());
                    let _ = stream
                        .write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n")
                        .await;
                });
            }
        });
        Self { url, heads }
    }

    pub(super) fn heads(&self) -> Vec<String> {
        self.heads.lock().unwrap().clone()
    }

    pub(super) fn plan(&self, credentials: &str, no_proxy: &str) -> ProxyPlan {
        let proxy = self
            .url
            .replacen("http://", &format!("http://{credentials}"), 1);
        ProxyPlan::Through(ProxyRoute::new(url(&proxy), no_proxy))
    }
}

fn limits() -> FetchLimits {
    FetchLimits {
        max_bytes: 1024 * 1024,
        timeout: Duration::from_secs(3),
        connect_timeout: Duration::from_millis(500),
    }
}

fn project(directory: &TempDir, name: &str, base_url: &str) -> Project {
    let store = ProjectStore::new(directory.paths());
    store.add(name, base_url, TOKEN).unwrap();
    store
        .load()
        .unwrap()
        .into_iter()
        .find(|project| project.name == name)
        .unwrap()
}

// The proxy sees only the CONNECT line, never the token.
#[tokio::test]
async fn https_reads_go_through_the_proxy_as_a_tunnel() {
    let stub = ProxyStub::start().await;
    let directory = TempDir::new("proxy-tunnel");
    let client = MonitorClient::with_proxy(limits(), &stub.plan("ops:s3cret@", "")).unwrap();
    let shop = project(&directory, "shop", "https://shop.example/");
    assert_eq!(
        client.fetch(&shop).await.unwrap_err(),
        FetchError::Unreachable
    );
    let heads = stub.heads();
    assert_eq!(heads.len(), 1, "{heads:?}");
    assert!(
        heads[0].starts_with("CONNECT shop.example:443 HTTP/1.1\r\n"),
        "{heads:?}"
    );
    // Basic `ops:s3cret`.
    assert!(heads[0].contains("Basic b3BzOnMzY3JldA=="), "{heads:?}");
    assert!(!heads[0].contains(TOKEN));
    assert!(!heads[0].contains("_rullst"));
}

#[tokio::test]
async fn loopback_and_no_proxy_hosts_are_reached_directly() {
    let stub = ProxyStub::start().await;
    let directory = TempDir::new("proxy-direct");
    let client = MonitorClient::with_proxy(limits(), &stub.plan("", "192.0.2.0/24")).unwrap();
    let refused = super::server_test::RefusedPort::new();
    for (name, base) in [
        ("tunnel", format!("http://127.0.0.1:{}/", refused.port)),
        ("local", format!("https://127.0.0.1:{}/", refused.port)),
        ("listed", "https://192.0.2.1/".to_string()),
    ] {
        let outcome = client.fetch(&project(&directory, name, &base)).await;
        assert!(outcome.is_err(), "{name}");
    }
    assert!(stub.heads().is_empty(), "{:?}", stub.heads());
}

#[tokio::test]
async fn an_unusable_proxy_sends_nothing() {
    let directory = TempDir::new("proxy-invalid");
    let client =
        MonitorClient::with_proxy(limits(), &ProxyPlan::Invalid(Unusable::NewerMode)).unwrap();
    let shop = project(&directory, "shop", "https://shop.example/");
    assert_eq!(
        client.fetch(&shop).await.unwrap_err(),
        FetchError::ProxyUnusable
    );
    assert_eq!(
        FetchError::ProxyUnusable.message(),
        "the proxy setting cannot be used; fix it on the Settings page"
    );
    let payload = WebhookPayload {
        source: "rullst-studio",
        project: "shop",
        rule: "unreachable",
        message: "shop could not be read",
        at: 1,
    };
    let webhooks = WebhookClient::new(&ProxyPlan::Invalid(Unusable::Socks("HTTPS_PROXY"))).unwrap();
    assert_eq!(
        webhooks.deliver("https://hooks.example/x", &payload).await,
        "failed: the proxy setting cannot be used"
    );
    let stub = ProxyStub::start().await;
    let through = WebhookClient::with_timeout(Duration::from_secs(3), &stub.plan("", "")).unwrap();
    assert_eq!(
        through.deliver("https://hooks.example/x", &payload).await,
        "failed: unreachable"
    );
    assert!(stub.heads()[0].starts_with("CONNECT hooks.example:443 HTTP/1.1\r\n"));
}
