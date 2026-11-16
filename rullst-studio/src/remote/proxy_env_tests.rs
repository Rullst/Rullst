//! The environment's proxy variables: chosen by target scheme, upper case
//! first, SOCKS and malformed values refused by name (`TM-STUDIO-3`,
//! `REMOTE-13`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::proxy::{ProxyMode, ProxyPlan, ProxyRoute, ProxySettings, Unusable};
use super::proxy_tests::{proxied, url};
use reqwest::Url;

fn environment(vars: &[(&str, &str)]) -> ProxyPlan {
    let settings = ProxySettings {
        mode: ProxyMode::Environment,
        ..ProxySettings::default()
    };
    ProxyPlan::resolve(&settings, |name| {
        vars.iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value.to_string())
    })
}

pub(super) fn through(plan: &ProxyPlan) -> &ProxyRoute {
    match plan {
        ProxyPlan::Through(route) => route,
        other => panic!("{other:?}"),
    }
}

/// The proxies of an `https://` and an `http://` request to `shop.example`.
pub(super) fn proxies(plan: &ProxyPlan) -> (Option<Url>, Option<Url>) {
    let route = through(plan);
    (
        route.proxy_for(&url("https://shop.example/")),
        route.proxy_for(&url("http://shop.example/")),
    )
}

#[test]
fn the_environment_proxy_follows_the_target_scheme() {
    assert_eq!(environment(&[]), ProxyPlan::Direct);
    assert_eq!(
        environment(&[("HTTPS_PROXY", " "), ("NO_PROXY", "x")]),
        ProxyPlan::Direct
    );
    let some = |value: &str| Some(url(value));
    assert_eq!(
        proxies(&environment(&[("HTTPS_PROXY", "http://s:1")])),
        (some("http://s:1"), None)
    );
    let http_only = environment(&[("HTTP_PROXY", "http://h:1")]);
    assert_eq!(proxies(&http_only), (None, some("http://h:1")));
    assert_eq!(
        http_only.describe(),
        "https:// connections are direct: only HTTP_PROXY is set, and Studio's http:// connections go to loopback, which is always direct."
    );
    assert_eq!(
        proxies(&environment(&[("ALL_PROXY", "http://a:1")])),
        (some("http://a:1"), some("http://a:1"))
    );
    assert_eq!(
        proxies(&environment(&[
            ("ALL_PROXY", "http://a:1"),
            ("HTTPS_PROXY", "http://s:1"),
        ])),
        (some("http://s:1"), some("http://a:1"))
    );
    assert_eq!(
        proxies(&environment(&[
            ("all_proxy", "http://a:1"),
            ("http_proxy", "h:2"),
        ])),
        (some("http://a:1"), some("http://h:2"))
    );
    // Upper case first, then lower case; a blank variable is unset.
    assert_eq!(
        proxies(&environment(&[
            ("HTTPS_PROXY", "http://user:secret@upper:3128"),
            ("https_proxy", "http://lower:3128"),
            ("HTTP_PROXY", ""),
            ("http_proxy", "http://lower-h:1"),
            ("ALL_PROXY", "http://upper-a:1"),
            ("all_proxy", "http://lower-a:1"),
        ])),
        (
            some("http://user:secret@upper:3128"),
            some("http://lower-h:1")
        )
    );
    // A variable that no target uses is not read.
    assert!(matches!(
        environment(&[
            ("HTTPS_PROXY", "http://s:1"),
            ("HTTP_PROXY", "http://h:1"),
            ("ALL_PROXY", "socks5://a:1080"),
        ]),
        ProxyPlan::Through(_)
    ));
}

#[test]
fn the_environment_exceptions_apply_to_both_schemes() {
    let plan = environment(&[
        ("ALL_PROXY", "http://a:1"),
        ("NO_PROXY", "skip.example"),
        ("no_proxy", "other.example"),
    ]);
    let route = through(&plan);
    assert!(!proxied(route, "https://skip.example/"));
    assert!(!proxied(route, "http://skip.example/"));
    assert!(proxied(route, "https://other.example/"));
    let lower = environment(&[("HTTPS_PROXY", "http://s:1"), ("no_proxy", "other.example")]);
    assert!(!proxied(through(&lower), "https://other.example/"));
    let longest = "a,".repeat(2_048);
    assert!(matches!(
        environment(&[("HTTPS_PROXY", "http://p:1"), ("NO_PROXY", &longest)]),
        ProxyPlan::Through(_)
    ));
    let long = format!("{longest}a");
    assert_eq!(
        environment(&[("HTTPS_PROXY", "http://p:1"), ("NO_PROXY", &long)]),
        ProxyPlan::Invalid(Unusable::LongExceptions("NO_PROXY"))
    );
    assert_eq!(
        environment(&[("HTTP_PROXY", "http://p:1"), ("no_proxy", &long)]),
        ProxyPlan::Invalid(Unusable::LongExceptions("no_proxy"))
    );
}

#[test]
fn a_socks_or_malformed_variable_is_refused_by_name() {
    for name in [
        "HTTPS_PROXY",
        "https_proxy",
        "HTTP_PROXY",
        "http_proxy",
        "ALL_PROXY",
        "all_proxy",
    ] {
        for (value, unusable) in [
            ("socks5://proxy:1080", Unusable::Socks(name)),
            ("socks5h://user:secret@proxy", Unusable::Socks(name)),
            ("socks4://proxy", Unusable::Socks(name)),
            ("http://proxy:3128/path", Unusable::Malformed(name)),
            ("ftp://proxy", Unusable::Malformed(name)),
        ] {
            let plan = environment(&[(name, value)]);
            assert_eq!(plan, ProxyPlan::Invalid(unusable), "{name}={value}");
            assert!(plan.blocks());
            assert!(!plan.describe().contains("secret"));
        }
    }
    // Either scheme's variable blocks every request.
    assert_eq!(
        environment(&[
            ("HTTPS_PROXY", "http://s:1"),
            ("HTTP_PROXY", "socks5://h:1")
        ]),
        ProxyPlan::Invalid(Unusable::Socks("HTTP_PROXY"))
    );
    assert_eq!(
        environment(&[
            ("HTTPS_PROXY", "socks5://s:1"),
            ("HTTP_PROXY", "socks5://h:1")
        ]),
        ProxyPlan::Invalid(Unusable::Socks("HTTPS_PROXY"))
    );
    assert!(!ProxyPlan::Direct.blocks());
    assert!(!environment(&[("ALL_PROXY", "http://a:1")]).blocks());
}
