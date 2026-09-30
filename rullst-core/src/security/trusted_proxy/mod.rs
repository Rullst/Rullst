//! Explicit trusted reverse-proxy client resolution.
//!
//! The socket peer (`ConnectInfo<SocketAddr>`) is the default client identity.
//! [`TrustedProxyLayer`] replaces it with the forwarded client address only for
//! connections whose socket peer is inside a configured trusted network, so the
//! rate limiters, Traffic Shield, the error console, Security middleware and
//! Nexus lockout all observe the real client without per-application code.

mod config;
mod parse;

#[cfg(test)]
mod layer_tests;
#[cfg(test)]
mod tests;

pub use config::{ForwardedHeader, TrustedProxyConfig, TrustedProxyError};

use axum::extract::{ConnectInfo, connect_info::MockConnectInfo};
use axum::http::{HeaderMap, Request};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::task::{Context, Poll};
use tower_layer::Layer;
use tower_service::Service;

/// Scheme a trusted proxy reported for the client connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ForwardedProto {
    /// The proxy received plaintext HTTP.
    Http,
    /// The proxy terminated TLS (HTTPS).
    Https,
}

impl ForwardedProto {
    /// Whether the proxy reported HTTPS.
    pub const fn is_https(self) -> bool {
        matches!(self, Self::Https)
    }

    /// Lowercase scheme name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Https => "https",
        }
    }
}

/// Client identity inserted as a request extension by [`TrustedProxyLayer`].
///
/// It has no public constructor: its presence proves the layer evaluated the
/// request against its explicit policy. Read it with
/// `axum::Extension<ClientAddr>` or `request.extensions().get::<ClientAddr>()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientAddr {
    ip: IpAddr,
    peer: SocketAddr,
    via_trusted_proxy: bool,
    forwarded_proto: Option<ForwardedProto>,
}

impl ClientAddr {
    /// Resolved client address, with IPv4-mapped IPv6 normalized to IPv4.
    ///
    /// This is the forwarded client when [`Self::via_trusted_proxy`] is
    /// `true`, otherwise the socket peer.
    pub const fn ip(&self) -> IpAddr {
        self.ip
    }

    /// Original socket peer of the connection, unchanged by the layer.
    pub const fn peer(&self) -> SocketAddr {
        self.peer
    }

    /// Whether [`Self::ip`] came from the forwarding header of a trusted peer.
    pub const fn via_trusted_proxy(&self) -> bool {
        self.via_trusted_proxy
    }

    /// Scheme reported by a trusted peer, present only when
    /// [`TrustedProxyConfig::trust_forwarded_proto`] is enabled and the value
    /// was unambiguous.
    pub const fn forwarded_proto(&self) -> Option<ForwardedProto> {
        self.forwarded_proto
    }
}

/// Tower layer that resolves the client address behind trusted proxies.
///
/// For a trusted socket peer with a usable forwarding header, the request's
/// `ConnectInfo<SocketAddr>` is replaced by the client IP with **port 0** (the
/// socket port belongs to the proxy connection, and a forwarded port is not
/// transport evidence). Every request with peer metadata also receives a
/// [`ClientAddr`] extension. A missing, malformed, oversized or over-long
/// header from a trusted peer keeps the socket peer, never fails the request,
/// and emits one `tracing` event without header contents. Untrusted peers are
/// never resolved through headers.
///
/// Mount it outside every middleware that reads the client address; `Server`
/// does this through [`crate::Server::trusted_proxies`].
#[derive(Debug, Clone)]
pub struct TrustedProxyLayer {
    config: Arc<TrustedProxyConfig>,
}

impl TrustedProxyLayer {
    /// Creates the layer from a validated policy.
    pub fn new(config: TrustedProxyConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    /// Policy applied by this layer.
    pub fn config(&self) -> &TrustedProxyConfig {
        &self.config
    }
}

impl<S> Layer<S> for TrustedProxyLayer {
    type Service = TrustedProxyService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        TrustedProxyService {
            inner,
            config: Arc::clone(&self.config),
        }
    }
}

/// Service produced by [`TrustedProxyLayer`].
#[derive(Debug, Clone)]
pub struct TrustedProxyService<S> {
    inner: S,
    config: Arc<TrustedProxyConfig>,
}

impl<S, B> Service<Request<B>> for TrustedProxyService<S>
where
    S: Service<Request<B>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut request: Request<B>) -> Self::Future {
        apply(&self.config, &mut request);
        self.inner.call(request)
    }
}

fn apply<B>(config: &TrustedProxyConfig, request: &mut Request<B>) {
    // Same lookup order as Axum's `ConnectInfo` extractor.
    let extensions = request.extensions();
    let Some(peer) = extensions
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(peer)| *peer)
        .or_else(|| {
            extensions
                .get::<MockConnectInfo<SocketAddr>>()
                .map(|MockConnectInfo(peer)| *peer)
        })
    else {
        return;
    };
    let client = resolve(config, peer, request.headers());
    if client.via_trusted_proxy {
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::new(client.ip, 0)));
    }
    request.extensions_mut().insert(client);
}

fn resolve(config: &TrustedProxyConfig, peer: SocketAddr, headers: &HeaderMap) -> ClientAddr {
    let peer_ip = peer.ip().to_canonical();
    let direct = ClientAddr {
        ip: peer_ip,
        peer,
        via_trusted_proxy: false,
        forwarded_proto: None,
    };
    if !config.is_trusted(peer_ip) {
        return direct;
    }
    let header_proto = match config.header {
        ForwardedHeader::XForwardedFor if config.trust_forwarded_proto => {
            parse::x_forwarded_proto(headers)
        }
        _ => None,
    };
    match parse::resolve_chain(config, headers) {
        Ok(resolution) => ClientAddr {
            ip: resolution.ip,
            peer,
            via_trusted_proxy: true,
            forwarded_proto: if config.trust_forwarded_proto {
                resolution.proto.or(header_proto)
            } else {
                None
            },
        },
        Err(rejection) => {
            report(rejection, config.header, peer_ip);
            ClientAddr {
                forwarded_proto: header_proto,
                ..direct
            }
        }
    }
}

/// Emits one bounded event; header contents are never recorded.
fn report(rejection: parse::Rejection, header: ForwardedHeader, peer: IpAddr) {
    if rejection == parse::Rejection::Missing {
        tracing::debug!(
            header = header.as_str(),
            peer = %peer,
            "trusted proxy sent no client address; keeping the socket peer"
        );
    } else {
        tracing::warn!(
            header = header.as_str(),
            reason = rejection.as_str(),
            peer = %peer,
            "ignored unusable forwarding metadata from a trusted proxy; keeping the socket peer"
        );
    }
}
