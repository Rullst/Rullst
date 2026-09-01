//! HTTPS GET transport that mounts [`super::EgressPolicy`] around every network hop.

use super::{EgressPolicy, EgressPolicyError, ValidatedEgressUrl};
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::header::{CONTENT_TYPE, LOCATION};
use std::collections::HashSet;
use std::fmt::Display;
use std::net::{IpAddr, SocketAddr};

/// Failure returned by the policy-bound outbound fetcher.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum EgressFetchError {
    /// The destination, redirect or resource budget violated policy.
    #[error("outbound request rejected by egress policy: {0}")]
    Policy(#[from] EgressPolicyError),
    /// DNS resolution exceeded the request deadline.
    #[error("outbound DNS resolution timed out")]
    ResolutionTimeout,
    /// DNS resolution failed without exposing the requested URL.
    #[error("outbound DNS resolution failed: {0}")]
    Resolution(String),
    /// The pinned HTTP client could not be constructed.
    #[error("outbound HTTP client construction failed")]
    ClientBuild,
    /// The network request or response body failed.
    #[error("outbound HTTP transport failed")]
    Transport,
    /// A redirect omitted a valid `Location` header.
    #[error("outbound redirect has no valid location")]
    InvalidRedirect,
    /// The transport did not expose the connected peer address.
    #[error("outbound transport did not expose its peer address")]
    PeerAddressUnavailable,
    /// The connected peer was not one of the validated, pinned DNS answers.
    #[error("outbound peer differs from the validated DNS answers")]
    PeerAddressMismatch,
    /// The origin returned a non-success response.
    #[error("outbound origin returned HTTP status {0}")]
    HttpStatus(u16),
    /// Memory reservation for the bounded response failed.
    #[error("outbound response buffer could not be reserved")]
    BufferAllocation,
}

/// Resolver contract used by [`EgressFetcher`].
///
/// Static dispatch keeps custom deployment resolvers explicit. Every returned
/// address is still checked by `EgressPolicy`; a resolver cannot waive policy.
#[async_trait]
pub trait EgressResolver: Send + Sync {
    /// Resolver-specific error kept outside the fetcher's public failure enum.
    type Error: Display + Send;

    /// Resolves all addresses for one normalized host and HTTPS port.
    async fn resolve(&self, host: &str, port: u16) -> Result<Vec<IpAddr>, Self::Error>;
}

/// Tokio system-DNS resolver used by the production fetcher constructor.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemEgressResolver;

#[async_trait]
impl EgressResolver for SystemEgressResolver {
    type Error = std::io::Error;

    async fn resolve(&self, host: &str, port: u16) -> Result<Vec<IpAddr>, Self::Error> {
        let mut unique = HashSet::new();
        let mut addresses = Vec::new();
        for address in tokio::net::lookup_host((host, port)).await? {
            if unique.insert(address.ip()) {
                addresses.push(address.ip());
            }
        }
        Ok(addresses)
    }
}

/// Bounded successful response from [`EgressFetcher::fetch_bytes`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchedResource {
    /// Final validated URL after manual redirects.
    pub final_url: String,
    /// Successful HTTP status.
    pub status: u16,
    /// Bounded textual content type when the origin supplied a valid header.
    pub content_type: Option<String>,
    /// Response bytes collected under the configured streaming limit.
    pub body: Vec<u8>,
}

/// HTTPS-only fetcher with explicit resolver, DNS pinning and resource budgets.
#[derive(Debug, Clone)]
pub struct EgressFetcher<R = SystemEgressResolver> {
    policy: EgressPolicy,
    resolver: R,
}

impl EgressFetcher<SystemEgressResolver> {
    /// Creates a fetcher with strict policy and the Tokio system resolver.
    pub fn strict() -> Self {
        Self {
            policy: EgressPolicy::strict(),
            resolver: SystemEgressResolver,
        }
    }

    /// Creates a system-resolver fetcher with an explicitly configured policy.
    pub fn new(policy: EgressPolicy) -> Self {
        Self {
            policy,
            resolver: SystemEgressResolver,
        }
    }
}

impl<R> EgressFetcher<R>
where
    R: EgressResolver,
{
    /// Creates a fetcher with an application-owned resolver implementation.
    pub fn with_resolver(policy: EgressPolicy, resolver: R) -> Self {
        Self { policy, resolver }
    }

    /// Fetches one HTTPS resource, manually validating and pinning every hop.
    pub async fn fetch_bytes(&self, input: &str) -> Result<FetchedResource, EgressFetchError> {
        let mut current = self.policy.validate_url(input)?;
        let mut redirects_followed = 0_usize;
        loop {
            let (host, port, addresses) = self.resolve_hop(&current).await?;
            let sockets = addresses
                .iter()
                .map(|address| SocketAddr::new(*address, port))
                .collect::<Vec<_>>();
            let mut builder = reqwest::Client::builder()
                .https_only(true)
                .no_proxy()
                .referer(false)
                .redirect(reqwest::redirect::Policy::none())
                .timeout(self.policy.request_timeout());
            if host.parse::<IpAddr>().is_err() {
                builder = builder.resolve_to_addrs(&host, &sockets);
            }
            let client = builder.build().map_err(|_| EgressFetchError::ClientBuild)?;
            let response = client
                .get(current.clone().into_url())
                .send()
                .await
                .map_err(|_| EgressFetchError::Transport)?;
            let peer = response
                .remote_addr()
                .ok_or(EgressFetchError::PeerAddressUnavailable)?;
            if !addresses.contains(&peer.ip()) {
                return Err(EgressFetchError::PeerAddressMismatch);
            }
            if response.status().is_redirection() {
                let location = response
                    .headers()
                    .get(LOCATION)
                    .and_then(|value| value.to_str().ok())
                    .ok_or(EgressFetchError::InvalidRedirect)?;
                current = self
                    .policy
                    .validate_redirect(&current, location, redirects_followed)?;
                redirects_followed = redirects_followed
                    .checked_add(1)
                    .ok_or(EgressPolicyError::RedirectLimitExceeded)?;
                continue;
            }
            if !response.status().is_success() {
                return Err(EgressFetchError::HttpStatus(response.status().as_u16()));
            }
            self.policy
                .validate_content_length(response.content_length())?;
            let status = response.status().as_u16();
            let content_type = response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .filter(|value| value.len() <= 256)
                .map(ToOwned::to_owned);
            let mut body = Vec::new();
            let mut stream = response.bytes_stream();
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(|_| EgressFetchError::Transport)?;
                append_bounded(&mut body, &chunk, self.policy.max_response_bytes())?;
            }
            return Ok(FetchedResource {
                final_url: current.as_str().to_string(),
                status,
                content_type,
                body,
            });
        }
    }

    async fn resolve_hop(
        &self,
        url: &ValidatedEgressUrl,
    ) -> Result<(String, u16, Vec<IpAddr>), EgressFetchError> {
        let host = url.host_str().trim_matches(['[', ']']).to_string();
        let parsed =
            reqwest::Url::parse(url.as_str()).map_err(|_| EgressPolicyError::InvalidUrl)?;
        let port = parsed
            .port_or_known_default()
            .ok_or(EgressPolicyError::BlockedPort)?;
        let resolved = if let Ok(address) = host.parse::<IpAddr>() {
            vec![address]
        } else {
            tokio::time::timeout(
                self.policy.request_timeout(),
                self.resolver.resolve(&host, port),
            )
            .await
            .map_err(|_| EgressFetchError::ResolutionTimeout)?
            .map_err(|error| EgressFetchError::Resolution(error.to_string()))?
        };
        let addresses = self.policy.validate_resolved_ips(url, resolved)?;
        Ok((host, port, addresses))
    }
}

fn append_bounded(body: &mut Vec<u8>, chunk: &[u8], maximum: u64) -> Result<(), EgressFetchError> {
    let current = u64::try_from(body.len()).map_err(|_| EgressPolicyError::ResponseTooLarge)?;
    let incoming = u64::try_from(chunk.len()).map_err(|_| EgressPolicyError::ResponseTooLarge)?;
    if current
        .checked_add(incoming)
        .is_none_or(|total| total > maximum)
    {
        return Err(EgressPolicyError::ResponseTooLarge.into());
    }
    body.try_reserve(chunk.len())
        .map_err(|_| EgressFetchError::BufferAllocation)?;
    body.extend_from_slice(chunk);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};
    use std::time::Duration;

    #[derive(Clone)]
    struct FixedResolver(Vec<IpAddr>);

    #[async_trait]
    impl EgressResolver for FixedResolver {
        type Error = std::io::Error;

        async fn resolve(&self, _host: &str, _port: u16) -> Result<Vec<IpAddr>, Self::Error> {
            Ok(self.0.clone())
        }
    }

    #[derive(Clone)]
    struct FailingResolver;

    #[async_trait]
    impl EgressResolver for FailingResolver {
        type Error = std::io::Error;

        async fn resolve(&self, _host: &str, _port: u16) -> Result<Vec<IpAddr>, Self::Error> {
            Err(std::io::Error::other("deterministic DNS failure"))
        }
    }

    #[derive(Clone)]
    struct SlowResolver;

    #[async_trait]
    impl EgressResolver for SlowResolver {
        type Error = std::io::Error;

        async fn resolve(&self, _host: &str, _port: u16) -> Result<Vec<IpAddr>, Self::Error> {
            tokio::time::sleep(Duration::from_millis(50)).await;
            Ok(vec![IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34))])
        }
    }

    #[tokio::test]
    // TM-AI-05: a connector cannot proceed when any DNS answer crosses policy.
    async fn fetcher_rejects_private_or_mixed_dns_before_transport() {
        for addresses in [
            vec![IpAddr::V4(Ipv4Addr::LOCALHOST)],
            vec![
                IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)),
                IpAddr::V6(Ipv6Addr::LOCALHOST),
            ],
        ] {
            let fetcher = EgressFetcher::with_resolver(
                EgressPolicy::strict()
                    .with_allowed_hosts(["example.test"])
                    .expect("test allowlist")
                    .with_request_timeout(Duration::from_millis(50))
                    .expect("test timeout"),
                FixedResolver(addresses),
            );
            assert!(matches!(
                fetcher.fetch_bytes("https://example.test/resource").await,
                Err(EgressFetchError::Policy(
                    EgressPolicyError::NonPublicAddress
                ))
            ));
        }
    }

    #[test]
    fn streamed_body_budget_is_enforced_before_append() {
        let mut body = Vec::new();
        append_bounded(&mut body, b"1234", 5).expect("first bounded chunk");
        assert_eq!(body, b"1234");
        assert!(matches!(
            append_bounded(&mut body, b"56", 5),
            Err(EgressFetchError::Policy(
                EgressPolicyError::ResponseTooLarge
            ))
        ));
        assert_eq!(body, b"1234");
    }

    #[test]
    fn source_mounts_dns_pinning_peer_check_and_manual_redirects() {
        let source = include_str!("egress_fetch.rs");
        assert!(source.contains("resolve_to_addrs"));
        assert!(source.contains("remote_addr()"));
        assert!(source.contains("Policy::none()"));
        assert!(source.contains("no_proxy()"));
        assert!(source.contains("bytes_stream()"));
    }

    #[tokio::test]
    async fn hop_resolution_distinguishes_literal_error_empty_and_timeout_outcomes() {
        let public = IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34));
        let literal_policy = EgressPolicy::strict()
            .with_allowed_hosts(["93.184.216.34"])
            .unwrap();
        let literal_url = literal_policy
            .validate_url("https://93.184.216.34/resource")
            .unwrap();
        let literal = EgressFetcher::with_resolver(literal_policy, FailingResolver)
            .resolve_hop(&literal_url)
            .await
            .expect("literal address bypasses DNS resolver");
        assert_eq!(literal.0, "93.184.216.34");
        assert_eq!(literal.1, 443);
        assert_eq!(literal.2, vec![public]);

        let base_policy = || {
            EgressPolicy::strict()
                .with_allowed_hosts(["example.test"])
                .unwrap()
                .with_request_timeout(Duration::from_millis(5))
                .unwrap()
        };
        let empty_policy = base_policy();
        let empty_url = empty_policy
            .validate_url("https://example.test/resource")
            .unwrap();
        let empty = EgressFetcher::with_resolver(empty_policy, FixedResolver(Vec::new()))
            .resolve_hop(&empty_url)
            .await;
        assert!(matches!(
            empty,
            Err(EgressFetchError::Policy(
                EgressPolicyError::ResolutionRequired
            ))
        ));

        let failure_policy = base_policy();
        let failure_url = failure_policy
            .validate_url("https://example.test/resource")
            .unwrap();
        assert!(matches!(
            EgressFetcher::with_resolver(failure_policy, FailingResolver)
                .resolve_hop(&failure_url)
                .await,
            Err(EgressFetchError::Resolution(message)) if message.contains("deterministic DNS failure")
        ));

        let slow_policy = base_policy();
        let slow_url = slow_policy
            .validate_url("https://example.test/resource")
            .unwrap();
        assert!(matches!(
            EgressFetcher::with_resolver(slow_policy, SlowResolver)
                .resolve_hop(&slow_url)
                .await,
            Err(EgressFetchError::ResolutionTimeout)
        ));
    }

    #[tokio::test]
    async fn system_resolver_deduplicates_localhost_answers() {
        let addresses = SystemEgressResolver
            .resolve("localhost", 443)
            .await
            .expect("system localhost resolution");
        assert!(!addresses.is_empty());
        let unique = addresses.iter().copied().collect::<HashSet<_>>();
        assert_eq!(addresses.len(), unique.len());
    }

    #[test]
    fn constructors_and_error_messages_keep_policy_explicit() {
        let strict = EgressFetcher::strict();
        assert!(strict.policy.validate_url("https://example.com").is_err());
        let configured = EgressPolicy::strict()
            .with_allowed_hosts(["example.com"])
            .unwrap();
        let fetcher = EgressFetcher::new(configured);
        assert!(fetcher.policy.validate_url("https://example.com").is_ok());

        let errors = [
            EgressFetchError::ResolutionTimeout,
            EgressFetchError::Resolution("redacted".to_owned()),
            EgressFetchError::ClientBuild,
            EgressFetchError::Transport,
            EgressFetchError::InvalidRedirect,
            EgressFetchError::PeerAddressUnavailable,
            EgressFetchError::PeerAddressMismatch,
            EgressFetchError::HttpStatus(503),
            EgressFetchError::BufferAllocation,
        ];
        for error in errors {
            assert!(!error.to_string().is_empty());
        }
    }
}
