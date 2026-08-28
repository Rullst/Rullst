//! Strict destination and response-budget contract for policy-bound fetchers and connectors.
//!
//! Validation must be repeated after DNS resolution and for every redirect. A
//! caller must connect to one of the validated addresses rather than resolving
//! the hostname again, otherwise DNS rebinding can cross the checked boundary.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::Duration;

const DEFAULT_MAX_REDIRECTS: usize = 3;
const DEFAULT_MAX_RESPONSE_BYTES: u64 = 2 * 1024 * 1024;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// A URL that passed the syntactic portion of an [`EgressPolicy`].
///
/// Hostnames still require [`EgressPolicy::validate_resolved_ips`] immediately
/// before a connector opens the socket.
#[derive(Debug, Clone)]
pub struct ValidatedEgressUrl {
    url: reqwest::Url,
}

impl ValidatedEgressUrl {
    /// Returns the validated URL string.
    pub fn as_str(&self) -> &str {
        self.url.as_str()
    }

    /// Returns the normalized hostname without credentials.
    pub fn host_str(&self) -> &str {
        self.url.host_str().unwrap_or_default()
    }

    /// Consumes the wrapper for use by an HTTP connector after DNS validation.
    pub fn into_url(self) -> reqwest::Url {
        self.url
    }
}

/// Typed rejection from the strict outbound network policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum EgressPolicyError {
    #[error("outbound URL is malformed")]
    InvalidUrl,
    #[error("outbound URL must use HTTPS")]
    InsecureScheme,
    #[error("outbound URL must not contain credentials or a fragment")]
    UnsafeUrlComponents,
    #[error("outbound hostname is missing or blocked")]
    BlockedHost,
    #[error("outbound hostname is not in the exact allowlist")]
    HostNotAllowed,
    #[error("outbound port is not allowed")]
    BlockedPort,
    #[error("outbound address is private, local, reserved, or otherwise non-public")]
    NonPublicAddress,
    #[error("hostname must resolve to at least one validated public address")]
    ResolutionRequired,
    #[error("resolved address does not match the literal URL host")]
    ResolutionMismatch,
    #[error("redirect limit was exceeded")]
    RedirectLimitExceeded,
    #[error("response content length exceeds the configured budget")]
    ResponseTooLarge,
    #[error("egress policy configuration is invalid")]
    InvalidConfiguration,
}

/// Fail-closed SSRF and resource-budget foundation for HTTP fetchers.
#[derive(Debug, Clone)]
pub struct EgressPolicy {
    allowed_hosts: Vec<String>,
    allowed_ports: Vec<u16>,
    max_redirects: usize,
    max_response_bytes: u64,
    request_timeout: Duration,
}

impl Default for EgressPolicy {
    fn default() -> Self {
        Self {
            allowed_hosts: Vec::new(),
            allowed_ports: vec![443],
            max_redirects: DEFAULT_MAX_REDIRECTS,
            max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
            request_timeout: DEFAULT_TIMEOUT,
        }
    }
}

impl EgressPolicy {
    /// Creates the strict HTTPS-only policy with bounded redirects, body and time.
    pub fn strict() -> Self {
        Self::default()
    }

    /// Replaces the exact destination-host allowlist.
    ///
    /// `strict()` starts empty and therefore denies every host until this is
    /// configured. Wildcards and private/literal-local hosts are rejected.
    pub fn with_allowed_hosts<S, I>(mut self, hosts: I) -> Result<Self, EgressPolicyError>
    where
        S: Into<String>,
        I: IntoIterator<Item = S>,
    {
        let mut hosts = hosts
            .into_iter()
            .map(Into::into)
            .map(|host: String| super::egress_host::normalize_allowed_host(&host))
            .collect::<Result<Vec<_>, _>>()?;
        hosts.sort();
        hosts.dedup();
        if hosts.is_empty() || hosts.len() > 128 {
            return Err(EgressPolicyError::InvalidConfiguration);
        }
        self.allowed_hosts = hosts;
        Ok(self)
    }

    /// Replaces the allowed HTTPS port set.
    pub fn with_allowed_ports(
        mut self,
        ports: impl IntoIterator<Item = u16>,
    ) -> Result<Self, EgressPolicyError> {
        let mut ports = ports.into_iter().collect::<Vec<_>>();
        ports.sort_unstable();
        ports.dedup();
        if ports.is_empty() || ports.contains(&0) {
            return Err(EgressPolicyError::InvalidConfiguration);
        }
        self.allowed_ports = ports;
        Ok(self)
    }

    /// Sets the maximum number of redirects followed by the connector.
    pub const fn with_max_redirects(mut self, max_redirects: usize) -> Self {
        self.max_redirects = max_redirects;
        self
    }

    /// Sets the maximum buffered/streamed response bytes.
    pub fn with_max_response_bytes(
        mut self,
        max_response_bytes: u64,
    ) -> Result<Self, EgressPolicyError> {
        if max_response_bytes == 0 {
            return Err(EgressPolicyError::InvalidConfiguration);
        }
        self.max_response_bytes = max_response_bytes;
        Ok(self)
    }

    /// Sets the per-request deadline.
    pub fn with_request_timeout(
        mut self,
        request_timeout: Duration,
    ) -> Result<Self, EgressPolicyError> {
        if request_timeout.is_zero() {
            return Err(EgressPolicyError::InvalidConfiguration);
        }
        self.request_timeout = request_timeout;
        Ok(self)
    }

    /// Maximum redirects accepted by [`EgressPolicy::validate_redirect`].
    pub const fn max_redirects(&self) -> usize {
        self.max_redirects
    }

    /// Response byte budget a connector must enforce while streaming.
    pub const fn max_response_bytes(&self) -> u64 {
        self.max_response_bytes
    }

    /// Per-request deadline a connector must apply.
    pub const fn request_timeout(&self) -> Duration {
        self.request_timeout
    }

    /// Validates scheme, components, host, literal address and port.
    pub fn validate_url(&self, input: &str) -> Result<ValidatedEgressUrl, EgressPolicyError> {
        let url = reqwest::Url::parse(input).map_err(|_| EgressPolicyError::InvalidUrl)?;
        self.validate_parsed_url(url)
    }

    /// Resolves and validates one redirect location against the same strict policy.
    pub fn validate_redirect(
        &self,
        previous: &ValidatedEgressUrl,
        location: &str,
        redirects_followed: usize,
    ) -> Result<ValidatedEgressUrl, EgressPolicyError> {
        if redirects_followed >= self.max_redirects {
            return Err(EgressPolicyError::RedirectLimitExceeded);
        }
        let next = previous
            .url
            .join(location)
            .map_err(|_| EgressPolicyError::InvalidUrl)?;
        self.validate_parsed_url(next)
    }

    /// Rejects empty, mixed, private, local, metadata, multicast and reserved DNS answers.
    ///
    /// The connector must pin its socket to one of these exact validated
    /// addresses and repeat this check after every redirect.
    pub fn validate_resolved_ips(
        &self,
        url: &ValidatedEgressUrl,
        addresses: impl IntoIterator<Item = IpAddr>,
    ) -> Result<Vec<IpAddr>, EgressPolicyError> {
        let addresses = addresses.into_iter().collect::<Vec<_>>();
        if addresses.is_empty() {
            return Err(EgressPolicyError::ResolutionRequired);
        }
        if addresses.iter().any(|address| !is_public_ip(*address)) {
            return Err(EgressPolicyError::NonPublicAddress);
        }
        let host = url
            .host_str()
            .trim_matches(|character| matches!(character, '[' | ']'));
        if let Ok(literal) = host.parse::<IpAddr>()
            && addresses.iter().any(|address| *address != literal)
        {
            return Err(EgressPolicyError::ResolutionMismatch);
        }
        Ok(addresses)
    }

    /// Rejects a declared response size above the streaming byte budget.
    pub fn validate_content_length(
        &self,
        content_length: Option<u64>,
    ) -> Result<(), EgressPolicyError> {
        if content_length.is_some_and(|length| length > self.max_response_bytes) {
            return Err(EgressPolicyError::ResponseTooLarge);
        }
        Ok(())
    }

    fn validate_parsed_url(
        &self,
        url: reqwest::Url,
    ) -> Result<ValidatedEgressUrl, EgressPolicyError> {
        if url.scheme() != "https" {
            return Err(EgressPolicyError::InsecureScheme);
        }
        if !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() {
            return Err(EgressPolicyError::UnsafeUrlComponents);
        }

        let host = url.host_str().ok_or(EgressPolicyError::BlockedHost)?;
        let normalized_host = host
            .trim_matches(|character| matches!(character, '[' | ']'))
            .trim_end_matches('.')
            .to_ascii_lowercase();
        if normalized_host.is_empty()
            || normalized_host == "localhost"
            || normalized_host.ends_with(".localhost")
            || matches!(
                normalized_host.as_str(),
                "metadata.google.internal" | "instance-data"
            )
        {
            return Err(EgressPolicyError::BlockedHost);
        }
        if !self.allowed_hosts.contains(&normalized_host) {
            return Err(EgressPolicyError::HostNotAllowed);
        }
        let address_text = normalized_host
            .strip_prefix('[')
            .and_then(|host| host.strip_suffix(']'))
            .unwrap_or(&normalized_host);
        if let Ok(address) = address_text.parse::<IpAddr>()
            && !is_public_ip(address)
        {
            return Err(EgressPolicyError::NonPublicAddress);
        }

        let port = url
            .port_or_known_default()
            .ok_or(EgressPolicyError::BlockedPort)?;
        if !self.allowed_ports.contains(&port) {
            return Err(EgressPolicyError::BlockedPort);
        }

        Ok(ValidatedEgressUrl { url })
    }
}

pub(super) const fn is_public_ip(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => is_public_ipv4(address),
        IpAddr::V6(address) => is_public_ipv6(address),
    }
}

const fn is_public_ipv4(address: Ipv4Addr) -> bool {
    let [a, b, c, d] = address.octets();
    if a == 0
        || a == 10
        || a == 127
        || a >= 224
        || (a == 100 && b >= 64 && b <= 127)
        || (a == 169 && b == 254)
        || (a == 172 && b >= 16 && b <= 31)
        || (a == 192 && b == 168)
        || (a == 198 && matches!(b, 18 | 19))
        || (a == 192 && b == 0 && c == 0)
    {
        return false;
    }
    !matches!(
        (a, b, c, d),
        (192, 0, 2, _) | (198, 51, 100, _) | (203, 0, 113, _) | (255, 255, 255, 255)
    )
}

const fn is_public_ipv6(address: Ipv6Addr) -> bool {
    let segments = address.segments();
    if address.is_unspecified()
        || address.is_loopback()
        || (segments[0] & 0xe000) != 0x2000
        || (segments[0] == 0x2001 && segments[1] <= 0x01ff)
        || (segments[0] == 0x2001 && segments[1] == 0x0db8)
        || segments[0] == 0x2002
        || (segments[0] & 0xfff0) == 0x3ff0
    {
        return false;
    }
    match address.to_ipv4_mapped() {
        Some(address) => is_public_ipv4(address),
        None => true,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::{EgressPolicy, EgressPolicyError};
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    use std::time::Duration;

    #[test]
    // TM-AI-05: URL, DNS, redirect and resource checks deny common SSRF forms.
    fn strict_policy_rejects_ssrf_destination_forms() {
        let policy = EgressPolicy::strict()
            .with_allowed_hosts(["example.com", "93.184.216.34"])
            .expect("test allowlist");
        for url in [
            "http://example.com/",
            "https://user:password@example.com/",
            "https://example.com/#secret",
            "https://localhost/",
            "https://service.localhost/",
            "https://metadata.google.internal/",
            "https://127.0.0.1/",
            "https://2130706433/",
            "https://[::1]/",
            "https://10.10.10.10/",
            "https://169.254.169.254/latest/meta-data/",
            "https://example.com:8443/",
        ] {
            assert!(policy.validate_url(url).is_err(), "{url} must be blocked");
        }
    }

    #[test]
    fn hostname_resolution_must_be_nonempty_and_entirely_public() {
        let policy = EgressPolicy::strict()
            .with_allowed_hosts(["example.com", "93.184.216.34"])
            .expect("test allowlist");
        let url = policy
            .validate_url("https://example.com/resource")
            .expect("public hostname syntax");
        assert_eq!(url.host_str(), "example.com");
        assert_eq!(
            policy.validate_resolved_ips(&url, []),
            Err(EgressPolicyError::ResolutionRequired)
        );
        assert_eq!(
            policy.validate_resolved_ips(
                &url,
                [
                    IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)),
                    IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                ]
            ),
            Err(EgressPolicyError::NonPublicAddress)
        );
        assert!(
            policy
                .validate_resolved_ips(
                    &url,
                    [
                        IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)),
                        IpAddr::V6("2606:2800:220:1:248:1893:25c8:1946".parse().unwrap()),
                    ]
                )
                .is_ok()
        );

        let literal = policy
            .validate_url("https://93.184.216.34/")
            .expect("public literal URL");
        assert_eq!(
            policy.validate_resolved_ips(&literal, [IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))]),
            Err(EgressPolicyError::ResolutionMismatch)
        );
    }

    #[test]
    fn redirects_and_response_resources_remain_bounded() {
        let policy = EgressPolicy::strict()
            .with_allowed_hosts(["example.com"])
            .expect("test allowlist")
            .with_max_redirects(1)
            .with_max_response_bytes(1024)
            .expect("response budget")
            .with_request_timeout(Duration::from_secs(2))
            .expect("request timeout");
        let start = policy
            .validate_url("https://example.com/start")
            .expect("public start URL");
        assert!(policy.validate_redirect(&start, "/next", 0).is_ok());
        assert!(matches!(
            policy.validate_redirect(&start, "/again", 1),
            Err(EgressPolicyError::RedirectLimitExceeded)
        ));
        assert!(
            policy
                .validate_redirect(&start, "https://127.0.0.1/private", 0)
                .is_err()
        );
        assert!(matches!(
            policy.validate_redirect(&start, "https://attacker.example/exfiltrate", 0),
            Err(EgressPolicyError::HostNotAllowed)
        ));
        assert_eq!(policy.validate_content_length(Some(1024)), Ok(()));
        assert_eq!(
            policy.validate_content_length(Some(1025)),
            Err(EgressPolicyError::ResponseTooLarge)
        );
        assert_eq!(policy.max_response_bytes(), 1024);
        assert_eq!(policy.request_timeout(), Duration::from_secs(2));
    }

    #[test]
    fn reserved_ipv6_and_invalid_configuration_fail_closed() {
        let policy = EgressPolicy::strict()
            .with_allowed_hosts(["example.com"])
            .expect("test allowlist");
        let url = policy
            .validate_url("https://example.com/")
            .expect("public hostname syntax");
        for address in [
            Ipv6Addr::LOCALHOST,
            Ipv6Addr::UNSPECIFIED,
            "fc00::1".parse().unwrap(),
            "fe80::1".parse().unwrap(),
            "2001:db8::1".parse().unwrap(),
            "2001::1".parse().unwrap(),
            "2002:7f00:1::".parse().unwrap(),
            "3fff::1".parse().unwrap(),
            "::ffff:127.0.0.1".parse().unwrap(),
        ] {
            assert_eq!(
                policy.validate_resolved_ips(&url, [IpAddr::V6(address)]),
                Err(EgressPolicyError::NonPublicAddress)
            );
        }
        assert!(EgressPolicy::strict().with_allowed_ports([]).is_err());
        assert!(
            EgressPolicy::strict()
                .with_allowed_hosts(Vec::<String>::new())
                .is_err()
        );
        assert!(
            EgressPolicy::strict()
                .with_allowed_hosts(["*.example.com"])
                .is_err()
        );
        assert!(EgressPolicy::strict().with_max_response_bytes(0).is_err());
        assert!(
            EgressPolicy::strict()
                .with_request_timeout(Duration::ZERO)
                .is_err()
        );
    }
}
