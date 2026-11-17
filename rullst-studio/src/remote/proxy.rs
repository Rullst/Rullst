//! How the standalone Studio reaches monitored projects and webhooks:
//! directly (the default), through the proxy variables of its environment,
//! or through a proxy set on the Settings page.
//!
//! The environment's proxy is chosen by the target's scheme, as curl and
//! reqwest choose it: `HTTPS_PROXY` (else `ALL_PROXY`) for `https://`,
//! `HTTP_PROXY` (else `ALL_PROXY`) for `http://`, each upper case first, then
//! lower case, except for the hosts of `NO_PROXY`. Studio sends `http://` only
//! to loopback, and a loopback host is always reached directly, so in
//! practice only `https://` requests use a proxy, as an HTTP `CONNECT`
//! tunnel: the proxy relays TLS and never sees a token.
//!
//! Only `http://` and `https://` proxies are supported. A SOCKS proxy, a
//! malformed URL or an over-long exception list blocks every request, with a
//! status that names the variable or setting, instead of falling back to a
//! direct connection.

use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

const MAX_URL_CHARS: usize = 2_048;
const MAX_NO_PROXY_CHARS: usize = 4_096;
const MAX_NO_PROXY_ENTRIES: usize = 256;

/// Which proxy Studio uses.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProxyMode {
    /// Connect directly; proxy variables are ignored.
    #[default]
    Direct,
    /// The proxy variables of Studio's environment.
    Environment,
    /// The proxy URL and exceptions stored in the settings.
    Explicit,
    /// A mode saved by a newer Studio, kept as written. This Studio cannot
    /// apply it, so every request is refused until a mode it knows is saved.
    #[serde(untagged)]
    Newer(String),
}

/// The proxy settings, kept in the project list.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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

/// The proxy of each target scheme and the hosts reached without one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProxyRoute {
    https: Option<Url>,
    http: Option<Url>,
    bypass: Vec<Bypass>,
}

/// Why the chosen proxy cannot be used. Each names the environment variable
/// or the setting it comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Unusable {
    /// Not an `http://` or `https://` URL with a host and nothing else.
    Malformed(&'static str),
    /// A SOCKS proxy, which Studio does not support.
    Socks(&'static str),
    /// Exceptions longer than 4,096 characters.
    LongExceptions(&'static str),
    /// A mode saved by a newer Studio.
    NewerMode,
}

/// What Studio's HTTP clients do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProxyPlan {
    /// Connect directly.
    Direct,
    /// Use a proxy.
    Through(ProxyRoute),
    /// The chosen proxy cannot be used; every request is refused.
    Invalid(Unusable),
}

/// Why a proxy URL was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Refusal {
    Malformed,
    Socks,
}

impl Refusal {
    fn of(self, source: &'static str) -> Unusable {
        match self {
            Self::Malformed => Unusable::Malformed(source),
            Self::Socks => Unusable::Socks(source),
        }
    }
}

/// Validates a proxy URL: `http` or `https` (`http` when no scheme is given,
/// as curl assumes), a host, no path, query or fragment, and user
/// information only when `credentials` allows it (the environment's; a
/// stored proxy has none). A SOCKS URL is refused as such.
fn check_proxy_url(value: &str, credentials: bool) -> Result<Url, Refusal> {
    let value = value.trim();
    if value.len() > MAX_URL_CHARS {
        return Err(Refusal::Malformed);
    }
    let url = if value.contains("://") {
        Url::parse(value)
    } else {
        Url::parse(&format!("http://{value}"))
    }
    .map_err(|_| Refusal::Malformed)?;
    if url.scheme().starts_with("socks") {
        return Err(Refusal::Socks);
    }
    let valid = matches!(url.scheme(), "http" | "https")
        && url.host_str().is_some_and(|host| !host.is_empty())
        && matches!(url.path(), "" | "/")
        && url.query().is_none()
        && url.fragment().is_none()
        && (credentials || (url.username().is_empty() && url.password().is_none()));
    valid.then_some(url).ok_or(Refusal::Malformed)
}

/// [`check_proxy_url`] without the reason.
pub(crate) fn parse_proxy_url(value: &str, credentials: bool) -> Option<Url> {
    check_proxy_url(value, credentials).ok()
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

/// A proxy's scheme, host and port, for display (never its user
/// information).
fn shown(proxy: &Url) -> String {
    let host = proxy.host_str().unwrap_or_default();
    match proxy.port() {
        Some(port) => format!("{}://{host}:{port}", proxy.scheme()),
        None => format!("{}://{host}", proxy.scheme()),
    }
}

impl ProxyRoute {
    /// A route through `proxy` for both schemes, except for the hosts of
    /// `no_proxy`.
    pub(crate) fn new(proxy: Url, no_proxy: &str) -> Self {
        Self::by_scheme(Some(proxy.clone()), Some(proxy), no_proxy)
    }

    fn by_scheme(https: Option<Url>, http: Option<Url>, no_proxy: &str) -> Self {
        Self {
            https,
            http,
            bypass: parse_no_proxy(no_proxy),
        }
    }

    /// The proxy for a request to `target`, or `None` to connect directly.
    pub(crate) fn proxy_for(&self, target: &Url) -> Option<Url> {
        let proxy = match target.scheme() {
            "https" => self.https.as_ref(),
            "http" => self.http.as_ref(),
            _ => None,
        }?;
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
        (!loopback && !bypassed).then(|| proxy.clone())
    }
}

impl ProxyPlan {
    /// The plan for `settings`, reading the environment through `var`.
    pub(crate) fn resolve(settings: &ProxySettings, var: impl Fn(&str) -> Option<String>) -> Self {
        match &settings.mode {
            ProxyMode::Direct => Self::Direct,
            ProxyMode::Newer(_) => Self::Invalid(Unusable::NewerMode),
            ProxyMode::Environment => Self::from_environment(var),
            ProxyMode::Explicit => {
                let proxy = settings.url.as_deref().unwrap_or_default();
                match check_proxy_url(proxy, false) {
                    Ok(proxy) => Self::through(
                        ProxyRoute::new(proxy, &settings.no_proxy),
                        &settings.no_proxy,
                        "The saved exception list",
                    ),
                    Err(refusal) => Self::Invalid(refusal.of("The saved proxy URL")),
                }
            }
        }
    }

    /// The plan of the proxy variables read through `var`.
    fn from_environment(var: impl Fn(&str) -> Option<String>) -> Self {
        // The first of `names` that is set and not blank, with its name.
        let first = |names: [&'static str; 2]| {
            names.into_iter().find_map(|name| {
                var(name)
                    .filter(|value| !value.trim().is_empty())
                    .map(|value| (name, value))
            })
        };
        let all = first(["ALL_PROXY", "all_proxy"]);
        let https = first(["HTTPS_PROXY", "https_proxy"]).or_else(|| all.clone());
        let http = first(["HTTP_PROXY", "http_proxy"]).or(all);
        if https.is_none() && http.is_none() {
            return Self::Direct;
        }
        let parse = |selected: Option<(&'static str, String)>| {
            selected
                .map(|(name, value)| {
                    check_proxy_url(&value, true).map_err(|refusal| refusal.of(name))
                })
                .transpose()
        };
        let (https, http) = match (parse(https), parse(http)) {
            (Ok(https), Ok(http)) => (https, http),
            (Err(unusable), _) | (_, Err(unusable)) => return Self::Invalid(unusable),
        };
        let (source, no_proxy) =
            first(["NO_PROXY", "no_proxy"]).unwrap_or(("NO_PROXY", String::new()));
        Self::through(
            ProxyRoute::by_scheme(https, http, &no_proxy),
            &no_proxy,
            source,
        )
    }

    /// `route`, unless its exceptions (`no_proxy`, from `source`) are too
    /// long.
    fn through(route: ProxyRoute, no_proxy: &str, source: &'static str) -> Self {
        if no_proxy.len() > MAX_NO_PROXY_CHARS {
            return Self::Invalid(Unusable::LongExceptions(source));
        }
        Self::Through(route)
    }

    /// The plan for `settings` and Studio's own environment.
    pub(crate) fn from_settings(settings: &ProxySettings) -> Self {
        Self::resolve(settings, |name| std::env::var(name).ok())
    }

    /// Whether every request is refused.
    pub(crate) fn blocks(&self) -> bool {
        matches!(self, Self::Invalid(_))
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
            Self::Direct | Self::Invalid(_) => builder,
        }
    }

    /// One line describing the plan, for the Settings page.
    pub(crate) fn describe(&self) -> String {
        match self {
            Self::Direct => "Connections are direct.".to_string(),
            Self::Through(route) => match &route.https {
                Some(proxy) => format!(
                    "https:// connections go through {} as a CONNECT tunnel, except to loopback and the listed exceptions.",
                    shown(proxy)
                ),
                None => "https:// connections are direct: only HTTP_PROXY is set, and Studio's http:// connections go to loopback, which is always direct.".to_string(),
            },
            Self::Invalid(Unusable::Malformed(source)) => format!(
                "{source} is not a usable proxy URL (http:// or https:// with a host and port only), so Studio connects to nothing: fix it or choose another option."
            ),
            Self::Invalid(Unusable::Socks(source)) => format!(
                "{source} names a SOCKS proxy, which Studio does not support, so it connects to nothing: use an http:// or https:// proxy, or choose another option."
            ),
            Self::Invalid(Unusable::LongExceptions(source)) => format!(
                "{source} is longer than 4096 characters, so Studio connects to nothing: shorten it or choose another option."
            ),
            Self::Invalid(Unusable::NewerMode) => "A newer Studio saved a proxy choice this Studio does not know, so it connects to nothing: choose one here and save.".to_string(),
        }
    }
}

impl ProxySettings {
    /// Validates the settings of the Settings page: an explicit proxy needs
    /// an `http://` or `https://` URL without user information, the
    /// exceptions are bounded, and a mode of a newer Studio cannot be saved.
    pub(crate) fn valid(&self) -> bool {
        let url = match &self.mode {
            ProxyMode::Explicit => self
                .url
                .as_deref()
                .is_some_and(|url| parse_proxy_url(url, false).is_some()),
            ProxyMode::Direct | ProxyMode::Environment => true,
            ProxyMode::Newer(_) => false,
        };
        url && self.no_proxy.len() <= MAX_NO_PROXY_CHARS
    }
}
