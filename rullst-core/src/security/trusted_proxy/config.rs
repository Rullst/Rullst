//! Validated trusted-proxy network policy.

use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

/// Maximum number of trusted proxy networks accepted by one policy.
pub(super) const MAX_TRUSTED_NETWORKS: usize = 64;
/// Longest configuration entry echoed back in a validation error.
const MAX_ECHOED_ENTRY_CHARS: usize = 64;

/// Forwarding header read from a trusted proxy. Exactly one is used per policy.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ForwardedHeader {
    /// The de-facto `X-Forwarded-For` address list, paired with `X-Forwarded-Proto`.
    #[default]
    XForwardedFor,
    /// The standard RFC 7239 `Forwarded` header, including its `proto=` parameter.
    Forwarded,
}

impl ForwardedHeader {
    /// Lowercase header name used by this mode.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::XForwardedFor => "x-forwarded-for",
            Self::Forwarded => "forwarded",
        }
    }
}

impl FromStr for ForwardedHeader {
    type Err = TrustedProxyError;

    /// Accepts `x-forwarded-for` or `forwarded` in any ASCII case.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.eq_ignore_ascii_case("x-forwarded-for") {
            Ok(Self::XForwardedFor)
        } else if value.eq_ignore_ascii_case("forwarded") {
            Ok(Self::Forwarded)
        } else {
            Err(TrustedProxyError::InvalidHeader(excerpt(value)))
        }
    }
}

impl fmt::Display for ForwardedHeader {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Rejected trusted-proxy configuration.
///
/// Echoed entries are truncated to 64 characters with control characters
/// replaced, so a malformed value cannot inject log lines.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum TrustedProxyError {
    /// The entry is not an IP address or `address/prefix` CIDR network.
    #[error("trusted proxy network `{0}` must be an IPv4/IPv6 address or CIDR network")]
    InvalidNetwork(String),
    /// The CIDR address has bits set below its prefix (for example `10.0.0.1/8`).
    #[error("trusted proxy network `{0}` has host bits set; use the network address")]
    HostBitsSet(String),
    /// A `/0` network would trust every client-supplied forwarding header.
    #[error("trusted proxy network `{0}` trusts every address; list only the real proxy networks")]
    TrustsEveryAddress(String),
    /// The same network was listed more than once.
    #[error("duplicate trusted proxy network `{0}`")]
    DuplicateNetwork(String),
    /// More networks than the bounded policy accepts.
    #[error("at most {maximum} trusted proxy networks can be configured")]
    TooManyNetworks {
        /// Maximum accepted number of networks.
        maximum: usize,
    },
    /// The forwarding header name is not supported.
    #[error("forwarding header `{0}` must be `x-forwarded-for` or `forwarded`")]
    InvalidHeader(String),
}

/// One canonical trusted network. IPv4-mapped IPv6 networks are stored as IPv4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum ProxyNetwork {
    V4 { network: u32, prefix: u8 },
    V6 { network: u128, prefix: u8 },
}

impl ProxyNetwork {
    fn parse(entry: &str) -> Result<Self, TrustedProxyError> {
        let invalid = || TrustedProxyError::InvalidNetwork(excerpt(entry));
        let (address, prefix) = match entry.split_once('/') {
            Some((address, prefix)) => {
                if prefix.is_empty()
                    || prefix.len() > 3
                    || !prefix.bytes().all(|b| b.is_ascii_digit())
                {
                    return Err(invalid());
                }
                (address, Some(prefix.parse::<u8>().map_err(|_| invalid())?))
            }
            None => (entry, None),
        };
        let network = match address.parse::<IpAddr>().map_err(|_| invalid())? {
            IpAddr::V4(address) => {
                let prefix = prefix.unwrap_or(32);
                if prefix > 32 {
                    return Err(invalid());
                }
                Self::V4 {
                    network: u32::from(address),
                    prefix,
                }
            }
            IpAddr::V6(address) => {
                let prefix = prefix.unwrap_or(128);
                if prefix > 128 {
                    return Err(invalid());
                }
                match address.to_ipv4_mapped() {
                    Some(mapped) if prefix >= 96 => Self::V4 {
                        network: u32::from(mapped),
                        prefix: prefix - 96,
                    },
                    _ => Self::V6 {
                        network: u128::from(address),
                        prefix,
                    },
                }
            }
        };
        if network.prefix() == 0 {
            return Err(TrustedProxyError::TrustsEveryAddress(excerpt(entry)));
        }
        if !network.is_canonical() {
            return Err(TrustedProxyError::HostBitsSet(excerpt(entry)));
        }
        Ok(network)
    }

    const fn prefix(self) -> u8 {
        match self {
            Self::V4 { prefix, .. } | Self::V6 { prefix, .. } => prefix,
        }
    }

    fn is_canonical(self) -> bool {
        match self {
            Self::V4 { network, prefix } => network & v4_mask(prefix) == network,
            Self::V6 { network, prefix } => network & v6_mask(prefix) == network,
        }
    }

    /// Whether the canonical address belongs to this network.
    pub(super) fn contains(self, ip: IpAddr) -> bool {
        match (self, ip) {
            (Self::V4 { network, prefix }, IpAddr::V4(ip)) => {
                u32::from(ip) & v4_mask(prefix) == network
            }
            (Self::V6 { network, prefix }, IpAddr::V6(ip)) => {
                u128::from(ip) & v6_mask(prefix) == network
            }
            _ => false,
        }
    }
}

impl fmt::Display for ProxyNetwork {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::V4 { network, prefix } => {
                write!(formatter, "{}/{prefix}", Ipv4Addr::from(network))
            }
            Self::V6 { network, prefix } => {
                write!(formatter, "{}/{prefix}", Ipv6Addr::from(network))
            }
        }
    }
}

fn v4_mask(prefix: u8) -> u32 {
    u32::MAX.checked_shl(32 - u32::from(prefix)).unwrap_or(0)
}

fn v6_mask(prefix: u8) -> u128 {
    u128::MAX.checked_shl(128 - u32::from(prefix)).unwrap_or(0)
}

/// Bounded, validated description of the reverse proxies allowed to report
/// the client address.
///
/// Only a request whose socket peer (`ConnectInfo<SocketAddr>`) is inside one
/// of these networks has its forwarding header read. An empty policy (the
/// default) never reads forwarding headers: there is no implicit "trust all".
/// List only the networks your own proxies connect from; any other host in a
/// listed network can choose the client identity.
///
/// ```
/// use rullst_core::security::{ForwardedHeader, TrustedProxyConfig};
///
/// let proxies = TrustedProxyConfig::new(["10.0.0.0/8", "::1"])?
///     .with_header(ForwardedHeader::XForwardedFor)
///     .trust_forwarded_proto(true);
/// assert!(proxies.is_trusted("10.1.2.3".parse()?));
/// assert!(!proxies.is_trusted("203.0.113.9".parse()?));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Default, PartialEq, Eq)]
pub struct TrustedProxyConfig {
    pub(super) networks: Vec<ProxyNetwork>,
    pub(super) header: ForwardedHeader,
    pub(super) trust_forwarded_proto: bool,
}

impl TrustedProxyConfig {
    /// Maximum number of trusted networks in one policy.
    pub const MAX_NETWORKS: usize = MAX_TRUSTED_NETWORKS;
    /// Maximum forwarding-header bytes examined, counted from the right end.
    pub const MAX_HEADER_BYTES: usize = super::parse::MAX_EXAMINED_BYTES;
    /// Maximum address entries examined, counted from the right end.
    pub const MAX_HOPS: usize = super::parse::MAX_EXAMINED_HOPS;

    /// Validates a list of trusted proxy addresses or CIDR networks.
    ///
    /// Each entry is an IPv4/IPv6 address (a single host) or an
    /// `address/prefix` network without host bits. IPv4-mapped IPv6 entries are
    /// normalized to IPv4. `/0`, duplicates and more than
    /// [`Self::MAX_NETWORKS`] entries are rejected. An empty list yields a
    /// disabled policy that never reads forwarding headers.
    pub fn new<I, S>(networks: I) -> Result<Self, TrustedProxyError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut parsed = Vec::new();
        for entry in networks {
            if parsed.len() == MAX_TRUSTED_NETWORKS {
                return Err(TrustedProxyError::TooManyNetworks {
                    maximum: MAX_TRUSTED_NETWORKS,
                });
            }
            let entry = entry.as_ref();
            let network = ProxyNetwork::parse(entry)?;
            if parsed.contains(&network) {
                return Err(TrustedProxyError::DuplicateNetwork(excerpt(entry)));
            }
            parsed.push(network);
        }
        Ok(Self {
            networks: parsed,
            header: ForwardedHeader::default(),
            trust_forwarded_proto: false,
        })
    }

    /// Selects the single forwarding header read from trusted peers.
    ///
    /// The default is [`ForwardedHeader::XForwardedFor`]. The two headers are
    /// never merged; the unselected one is ignored.
    pub fn with_header(mut self, header: ForwardedHeader) -> Self {
        self.header = header;
        self
    }

    /// Also reports the scheme a trusted proxy received (`X-Forwarded-Proto`
    /// or `Forwarded: proto=`) through [`super::ClientAddr::forwarded_proto`].
    ///
    /// Enable this only when every trusted proxy overwrites that value, so a
    /// client cannot choose it. The default is `false`.
    pub fn trust_forwarded_proto(mut self, enabled: bool) -> Self {
        self.trust_forwarded_proto = enabled;
        self
    }

    /// Whether at least one trusted network is configured.
    pub fn is_enabled(&self) -> bool {
        !self.networks.is_empty()
    }

    /// Whether `ip` (after IPv4-mapped normalization) is a trusted proxy address.
    pub fn is_trusted(&self, ip: IpAddr) -> bool {
        let ip = ip.to_canonical();
        self.networks.iter().any(|network| network.contains(ip))
    }

    /// Forwarding header read from trusted peers.
    pub fn header(&self) -> ForwardedHeader {
        self.header
    }

    /// Whether the forwarded scheme from trusted peers is reported.
    pub fn trusts_forwarded_proto(&self) -> bool {
        self.trust_forwarded_proto
    }

    /// Builds the policy from the `[security]` settings of `Rullst.toml`
    /// (`trusted_proxies`, `trusted_proxy_header`, `trust_forwarded_proto`).
    pub fn from_security_config(
        security: &crate::config::SecurityConfig,
    ) -> Result<Self, TrustedProxyError> {
        Ok(Self::new(&security.trusted_proxies)?
            .with_header(security.trusted_proxy_header.parse()?)
            .trust_forwarded_proto(security.trust_forwarded_proto))
    }
}

impl fmt::Debug for TrustedProxyConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let networks = self
            .networks
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        formatter
            .debug_struct("TrustedProxyConfig")
            .field("networks", &networks)
            .field("header", &self.header)
            .field("trust_forwarded_proto", &self.trust_forwarded_proto)
            .finish()
    }
}

fn excerpt(value: &str) -> String {
    value
        .chars()
        .take(MAX_ECHOED_ENTRY_CHARS)
        .map(|character| {
            if character.is_control() {
                '?'
            } else {
                character
            }
        })
        .collect()
}
