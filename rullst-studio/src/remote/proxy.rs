//! How the standalone Studio reaches monitored projects and webhooks:
//! directly (the default), through the proxy of `HTTPS_PROXY` with the
//! exceptions of `NO_PROXY`, or through a proxy set on the Settings page.
//!
//! Only `https://` requests go through a proxy, as an HTTP `CONNECT` tunnel,
//! so the proxy relays TLS and never sees a token; plain `http://` requests
//! (which Studio sends only to loopback) and requests to a loopback host are
//! always direct. A proxy setting that cannot be used blocks every request
//! instead of falling back to a direct connection.

use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

const MAX_URL_CHARS: usize = 2_048;
const MAX_NO_PROXY_CHARS: usize = 4_096;
const MAX_NO_PROXY_ENTRIES: usize = 256;

/// Which proxy Studio uses.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProxyMode {
    /// Connect directly; proxy variables are ignored.
    #[default]
    Direct,
    /// `HTTPS_PROXY` (or `https_proxy`) and `NO_PROXY` (or `no_proxy`) of
    /// Studio's environment.
    Environment,
    /// The proxy URL and exceptions stored in the settings.
    Explicit,
}

/// The proxy settings, kept in the project list.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProxySettings {
    #[serde(default)]
    pub(crate) mode: ProxyMode,
    /// The proxy of [`ProxyMode::Explicit`], without user information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) url: Option<String>,
    /// Hosts reached directly in [`ProxyMode::Explicit`], in `NO_PROXY`
    /// syntax.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub(crate) no_proxy: String,
}

/// One `NO_PROXY` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Bypass {
    /// `*`: every host.
    All,
    /// A host name and its subdomains, lowercase, without a leading dot.
    Domain(String),
    /// An address range (an address alone is a full-length prefix).
    Network(IpAddr, u8),
}

/// A usable proxy and the hosts reached without it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProxyRoute {
    proxy: Url,
    bypass: Vec<Bypass>,
}

/// What Studio's HTTP clients do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProxyPlan {
    /// Connect directly.
    Direct,
    /// Use a proxy.
    Through(ProxyRoute),
    /// The configured proxy cannot be used; every request is refused.
    Invalid,
}

/// Validates a proxy URL: `http` or `https`, a host, no path, query or
/// fragment, and user information only when `credentials` allows it (the
/// environment's; a stored proxy has none).
pub(crate) fn parse_proxy_url(value: &str, credentials: bool) -> Option<Url> {
    let value = value.trim();
    if value.len() > MAX_URL_CHARS {
        return None;
    }
    let url = Url::parse(value).ok()?;
    let valid = matches!(url.scheme(), "http" | "https")
        && url.host_str().is_some_and(|host| !host.is_empty())
        && matches!(url.path(), "" | "/")
        && url.query().is_none()
        && url.fragment().is_none()
        && (credentials || (url.username().is_empty() && url.password().is_none()));
    valid.then_some(url)
}

/// The `NO_PROXY` entries of `value`, separated by commas or whitespace.
fn parse_no_proxy(value: &str) -> Vec<Bypass> {
    value
        .split(|character: char| character == ',' || character.is_whitespace())
        .take(MAX_NO_PROXY_ENTRIES)
        .filter_map(|entry| {
            if entry == "*" {
                return Some(Bypass::All);
            }
            if let Some((address, bits)) = entry.split_once('/')
                && let (Some(address), Ok(bits)) = (host_address(address), bits.parse::<u8>())
            {
                return Some(Bypass::Network(address, bits));
            }
            if let Some(address) = host_address(entry) {
                let bits = if address.is_ipv4() { 32 } else { 128 };
                return Some(Bypass::Network(address, bits));
            }
            let domain = entry.trim_start_matches('.').to_ascii_lowercase();
            (!domain.is_empty()).then_some(Bypass::Domain(domain))
        })
        .collect()
}

/// Whether the first `bits` bits of `left` and `right` are equal.
fn same_prefix(left: IpAddr, right: IpAddr, bits: u8) -> bool {
    let (left, right, width) = match (left, right) {
        (IpAddr::V4(left), IpAddr::V4(right)) => (
            u128::from(u32::from(left)),
            u128::from(u32::from(right)),
            32_u32,
        ),
        (IpAddr::V6(left), IpAddr::V6(right)) => (u128::from(left), u128::from(right), 128),
        _ => return false,
    };
    let bits = u32::from(bits).min(width);
    // Shifting by the full width keeps no bit: every address matches.
    let shift = width - bits;
    left.checked_shr(shift).unwrap_or(0) == right.checked_shr(shift).unwrap_or(0)
}

/// A URL host as an address, without the brackets of an IPv6 literal.
fn host_address(host: &str) -> Option<IpAddr> {
    host.trim_start_matches('[')
        .trim_end_matches(']')
        .parse()
        .ok()
}

impl ProxyRoute {
    /// A route through `proxy`, except for the hosts of `no_proxy`.
    pub(crate) fn new(proxy: Url, no_proxy: &str) -> Self {
        Self {
            proxy,
            bypass: parse_no_proxy(no_proxy),
        }
    }

    /// The proxy for a request to `target`, or `None` to connect directly.
    pub(crate) fn proxy_for(&self, target: &Url) -> Option<Url> {
        if target.scheme() != "https" {
            return None;
        }
        let host = target.host_str()?.to_ascii_lowercase();
        let address = host_address(&host);
        let loopback = host == "localhost"
            || host.ends_with(".localhost")
            || address.is_some_and(|address| address.to_canonical().is_loopback());
        let bypassed = self.bypass.iter().any(|entry| match entry {
            Bypass::All => true,
            Bypass::Domain(domain) => {
                host == *domain
                    || host
                        .strip_suffix(domain.as_str())
                        .is_some_and(|rest| rest.ends_with('.'))
            }
            Bypass::Network(network, bits) => {
                address.is_some_and(|address| same_prefix(address, *network, *bits))
            }
        });
        (!loopback && !bypassed).then(|| self.proxy.clone())
    }

    /// The proxy's scheme, host and port, for display (never its user
    /// information).
    pub(crate) fn display(&self) -> String {
        let host = self.proxy.host_str().unwrap_or_default();
        match self.proxy.port() {
            Some(port) => format!("{}://{host}:{port}", self.proxy.scheme()),
            None => format!("{}://{host}", self.proxy.scheme()),
        }
    }
}

impl ProxyPlan {
    /// The plan for `settings`, reading the environment through `var`.
    pub(crate) fn resolve(settings: &ProxySettings, var: impl Fn(&str) -> Option<String>) -> Self {
        let first = |names: [&str; 2]| {
            names
                .into_iter()
                .find_map(|name| var(name).filter(|value| !value.trim().is_empty()))
        };
        let (proxy, no_proxy, credentials) = match settings.mode {
            ProxyMode::Direct => return Self::Direct,
            ProxyMode::Environment => {
                let Some(proxy) = first(["HTTPS_PROXY", "https_proxy"]) else {
                    return Self::Direct;
                };
                (
                    proxy,
                    first(["NO_PROXY", "no_proxy"]).unwrap_or_default(),
                    true,
                )
            }
            ProxyMode::Explicit => (
                settings.url.clone().unwrap_or_default(),
                settings.no_proxy.clone(),
                false,
            ),
        };
        if no_proxy.len() > MAX_NO_PROXY_CHARS {
            return Self::Invalid;
        }
        match parse_proxy_url(&proxy, credentials) {
            Some(url) => Self::Through(ProxyRoute::new(url, &no_proxy)),
            None => Self::Invalid,
        }
    }

    /// The plan for `settings` and Studio's own environment.
    pub(crate) fn from_settings(settings: &ProxySettings) -> Self {
        Self::resolve(settings, |name| std::env::var(name).ok())
    }

    /// `builder` with this plan's proxy; never the reqwest defaults.
    pub(crate) fn apply(&self, builder: reqwest::ClientBuilder) -> reqwest::ClientBuilder {
        let builder = builder.no_proxy();
        match self {
            Self::Through(route) => {
                let route = route.clone();
                builder.proxy(reqwest::Proxy::custom(move |target| {
                    route.proxy_for(target)
                }))
            }
            Self::Direct | Self::Invalid => builder,
        }
    }

    /// One line describing the plan, for the Settings page.
    pub(crate) fn describe(&self) -> String {
        match self {
            Self::Direct => "Connections are direct.".to_string(),
            Self::Through(route) => format!(
                "https:// connections go through {} as a CONNECT tunnel, except to loopback and the listed exceptions.",
                route.display()
            ),
            Self::Invalid => {
                "The proxy cannot be used, so Studio connects to nothing: fix the setting or the environment.".to_string()
            }
        }
    }
}

impl ProxySettings {
    /// Validates the settings of the Settings page: an explicit proxy needs
    /// a valid URL without user information, and the exceptions are bounded.
    pub(crate) fn valid(&self) -> bool {
        let url = match self.mode {
            ProxyMode::Explicit => self
                .url
                .as_deref()
                .is_some_and(|url| parse_proxy_url(url, false).is_some()),
            ProxyMode::Direct | ProxyMode::Environment => true,
        };
        url && self.no_proxy.len() <= MAX_NO_PROXY_CHARS
    }
}
