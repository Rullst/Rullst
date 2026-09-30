//! Bounded right-to-left parsing of forwarding headers from a trusted peer.
//!
//! Only the right-most part of a chain is examined: the walk stops at the
//! first address outside the trusted networks. Text to its left was supplied
//! by the client and is never parsed, so a client cannot choose its identity
//! or force a fallback by padding its own forwarding header.

use super::ForwardedProto;
use super::config::{ForwardedHeader, TrustedProxyConfig};
use axum::http::{HeaderMap, HeaderName};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Forwarding-header bytes (including separators) examined from the right.
pub(super) const MAX_EXAMINED_BYTES: usize = 4096;
/// Non-empty address entries examined from the right.
pub(super) const MAX_EXAMINED_HOPS: usize = 32;
/// Longest accepted node, e.g. `[ffff:…:255.255.255.255]:65535`.
const MAX_NODE_BYTES: usize = 64;
/// Total `X-Forwarded-Proto` bytes accepted.
const MAX_PROTO_BYTES: usize = 32;

const X_FORWARDED_FOR: HeaderName = HeaderName::from_static("x-forwarded-for");
const X_FORWARDED_PROTO: HeaderName = HeaderName::from_static("x-forwarded-proto");
const FORWARDED: HeaderName = HeaderName::from_static("forwarded");

/// Why a trusted peer's forwarding metadata was not used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Rejection {
    /// No non-empty forwarding entry was present.
    Missing,
    /// An examined entry is not a valid address or RFC 7239 element.
    Malformed,
    /// The client entry is `unknown` or an obfuscated identifier.
    Unidentified,
    /// The examined part exceeds [`MAX_EXAMINED_BYTES`].
    Oversized,
    /// The trusted part of the chain exceeds [`MAX_EXAMINED_HOPS`].
    TooManyHops,
}

impl Rejection {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Malformed => "malformed",
            Self::Unidentified => "unidentified",
            Self::Oversized => "oversized",
            Self::TooManyHops => "too_many_hops",
        }
    }
}

/// Client entry selected from a trusted chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Resolution {
    /// Canonical client address.
    pub(super) ip: IpAddr,
    /// `proto=` of the selected `Forwarded` element (never set for X-Forwarded-For).
    pub(super) proto: Option<ForwardedProto>,
}

/// Selects the client from the configured forwarding header of a trusted peer.
pub(super) fn resolve_chain(
    config: &TrustedProxyConfig,
    headers: &HeaderMap,
) -> Result<Resolution, Rejection> {
    match config.header {
        ForwardedHeader::XForwardedFor => walk(config, headers, &X_FORWARDED_FOR, |node| {
            parse_x_forwarded_for_node(node).map(|ip| (ip, None))
        }),
        ForwardedHeader::Forwarded => walk(config, headers, &FORWARDED, parse_forwarded_element),
    }
}

/// Parses exactly one `http`/`https` value from `X-Forwarded-Proto`.
///
/// Several values (appended by more than one proxy or supplied by a client) are
/// ambiguous and yield `None`.
pub(super) fn x_forwarded_proto(headers: &HeaderMap) -> Option<ForwardedProto> {
    let mut found = None;
    let mut total = 0usize;
    for line in &headers.get_all(X_FORWARDED_PROTO) {
        total = total.saturating_add(line.len());
        if total > MAX_PROTO_BYTES {
            return None;
        }
        for raw in line.as_bytes().split(|byte| *byte == b',') {
            let token = trim_ows(raw);
            if token.is_empty() {
                continue;
            }
            if found.is_some() {
                return None;
            }
            found = Some(parse_proto(std::str::from_utf8(token).ok()?)?);
        }
    }
    found
}

fn walk(
    config: &TrustedProxyConfig,
    headers: &HeaderMap,
    name: &HeaderName,
    mut parse: impl FnMut(&[u8]) -> Result<(IpAddr, Option<ForwardedProto>), Rejection>,
) -> Result<Resolution, Rejection> {
    let mut examined = 0usize;
    let mut hops = 0usize;
    let mut leftmost = None;
    // Multiple header lines form one list in their received order.
    for line in headers.get_all(name).iter().rev() {
        for raw in line.as_bytes().rsplit(|byte| *byte == b',') {
            examined = examined.saturating_add(raw.len() + 1);
            if examined > MAX_EXAMINED_BYTES + 1 {
                return Err(Rejection::Oversized);
            }
            let entry = trim_ows(raw);
            if entry.is_empty() {
                continue;
            }
            hops += 1;
            if hops > MAX_EXAMINED_HOPS {
                return Err(Rejection::TooManyHops);
            }
            let (ip, proto) = parse(entry)?;
            let ip = ip.to_canonical();
            let resolution = Resolution { ip, proto };
            if !config.is_trusted(ip) {
                return Ok(resolution);
            }
            leftmost = Some(resolution);
        }
    }
    // Every examined hop is a trusted proxy: the left-most is the client.
    leftmost.ok_or(Rejection::Missing)
}

fn parse_x_forwarded_for_node(node: &[u8]) -> Result<IpAddr, Rejection> {
    let text = node_text(node)?;
    if text.eq_ignore_ascii_case("unknown") {
        return Err(Rejection::Unidentified);
    }
    parse_address(text, AddressSyntax::XForwardedFor)
}

fn parse_forwarded_element(element: &[u8]) -> Result<(IpAddr, Option<ForwardedProto>), Rejection> {
    let text = std::str::from_utf8(element).map_err(|_| Rejection::Malformed)?;
    let mut node = None;
    let mut proto = None;
    for_each_pair(text, |name, value| {
        let slot = if name.eq_ignore_ascii_case("for") {
            &mut node
        } else if name.eq_ignore_ascii_case("proto") {
            &mut proto
        } else {
            return Ok(());
        };
        // RFC 7239: a parameter must not occur more than once per element.
        if slot.replace(value).is_some() {
            return Err(Rejection::Malformed);
        }
        Ok(())
    })?;
    let node = node.ok_or(Rejection::Malformed)?;
    let text = node_text(node.as_bytes())?;
    if text.eq_ignore_ascii_case("unknown") || text.starts_with('_') {
        return Err(Rejection::Unidentified);
    }
    let ip = parse_address(text, AddressSyntax::Forwarded)?;
    Ok((ip, proto.and_then(parse_proto)))
}

/// Visits every `name=value` pair of one `Forwarded` element, unquoting values.
fn for_each_pair<'a>(
    element: &'a str,
    mut visit: impl FnMut(&'a str, &'a str) -> Result<(), Rejection>,
) -> Result<(), Rejection> {
    let mut start = 0;
    let mut quoted = false;
    for (index, byte) in element.bytes().enumerate() {
        match byte {
            b'"' => quoted = !quoted,
            b';' if !quoted => {
                visit_pair(&element[start..index], &mut visit)?;
                start = index + 1;
            }
            _ => {}
        }
    }
    if quoted {
        return Err(Rejection::Malformed);
    }
    visit_pair(&element[start..], &mut visit)
}

fn visit_pair<'a>(
    pair: &'a str,
    visit: &mut impl FnMut(&'a str, &'a str) -> Result<(), Rejection>,
) -> Result<(), Rejection> {
    let pair = pair.trim_matches([' ', '\t']);
    if pair.is_empty() {
        return Ok(());
    }
    let (name, value) = pair.split_once('=').ok_or(Rejection::Malformed)?;
    if name.is_empty() || !name.bytes().all(is_tchar) {
        return Err(Rejection::Malformed);
    }
    visit(name, unquote(value)?)
}

/// Returns a token or the content of a quoted string without escapes.
fn unquote(value: &str) -> Result<&str, Rejection> {
    if let Some(inner) = value.strip_prefix('"') {
        let inner = inner.strip_suffix('"').ok_or(Rejection::Malformed)?;
        let printable = |byte: u8| byte == b'\t' || (0x20..=0x7e).contains(&byte);
        if inner
            .bytes()
            .all(|byte| printable(byte) && byte != b'"' && byte != b'\\')
        {
            return Ok(inner);
        }
        return Err(Rejection::Malformed);
    }
    if !value.is_empty() && value.bytes().all(is_tchar) {
        Ok(value)
    } else {
        Err(Rejection::Malformed)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AddressSyntax {
    /// Also accepts a bare, unbracketed IPv6 address; ports must be numeric.
    XForwardedFor,
    /// RFC 7239 node: bracketed IPv6 only; the port may be obfuscated.
    Forwarded,
}

fn parse_address(text: &str, syntax: AddressSyntax) -> Result<IpAddr, Rejection> {
    if let Some(rest) = text.strip_prefix('[') {
        let (inside, after) = rest.split_once(']').ok_or(Rejection::Malformed)?;
        let ip = inside
            .parse::<Ipv6Addr>()
            .map_err(|_| Rejection::Malformed)?;
        validate_port_suffix(after, syntax)?;
        return Ok(IpAddr::V6(ip));
    }
    if let Ok(ip) = text.parse::<Ipv4Addr>() {
        return Ok(IpAddr::V4(ip));
    }
    if let Some((host, _)) = text.split_once(':')
        && let Ok(ip) = host.parse::<Ipv4Addr>()
    {
        validate_port_suffix(&text[host.len()..], syntax)?;
        return Ok(IpAddr::V4(ip));
    }
    if syntax == AddressSyntax::XForwardedFor
        && let Ok(ip) = text.parse::<Ipv6Addr>()
    {
        return Ok(IpAddr::V6(ip));
    }
    Err(Rejection::Malformed)
}

/// Accepts an empty suffix or `:port`; the port itself is never used.
fn validate_port_suffix(suffix: &str, syntax: AddressSyntax) -> Result<(), Rejection> {
    if suffix.is_empty() {
        return Ok(());
    }
    let port = suffix.strip_prefix(':').ok_or(Rejection::Malformed)?;
    let numeric = !port.is_empty()
        && port.len() <= 5
        && port.bytes().all(|byte| byte.is_ascii_digit())
        && port.parse::<u16>().is_ok();
    let obfuscated = syntax == AddressSyntax::Forwarded
        && port.len() > 1
        && port.starts_with('_')
        && port[1..]
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'));
    if numeric || obfuscated {
        Ok(())
    } else {
        Err(Rejection::Malformed)
    }
}

fn parse_proto(value: &str) -> Option<ForwardedProto> {
    if value.eq_ignore_ascii_case("https") {
        Some(ForwardedProto::Https)
    } else if value.eq_ignore_ascii_case("http") {
        Some(ForwardedProto::Http)
    } else {
        None
    }
}

fn node_text(node: &[u8]) -> Result<&str, Rejection> {
    if node.len() > MAX_NODE_BYTES {
        return Err(Rejection::Malformed);
    }
    std::str::from_utf8(node).map_err(|_| Rejection::Malformed)
}

fn trim_ows(bytes: &[u8]) -> &[u8] {
    let is_ows = |byte: &u8| matches!(byte, b' ' | b'\t');
    let start = bytes
        .iter()
        .position(|byte| !is_ows(byte))
        .unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|byte| !is_ows(byte))
        .map_or(start, |index| index + 1);
    &bytes[start..end]
}

/// RFC 9110 `tchar`.
fn is_tchar(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'!' | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'.'
                | b'^'
                | b'_'
                | b'`'
                | b'|'
                | b'~'
        )
}
