//! Browser-origin boundary for the explicitly enabled local development CMS.

use axum::{
    extract::Request,
    http::{Method, Uri, header, uri::Authority},
};

pub(super) fn allows(request: &Request) -> bool {
    let Some(authority) = local_authority(request) else {
        return false;
    };
    let mut origins = request.headers().get_all(header::ORIGIN).iter();
    match (origins.next(), origins.next()) {
        (Some(origin), None) => origin
            .to_str()
            .ok()
            .and_then(|origin| origin.parse::<Uri>().ok())
            .is_some_and(|origin| same_origin(&origin, &authority)),
        (None, None) => matches!(
            *request.method(),
            Method::GET | Method::HEAD | Method::OPTIONS
        ),
        _ => false,
    }
}

fn local_authority(request: &Request) -> Option<Authority> {
    let mut hosts = request.headers().get_all(header::HOST).iter();
    let header_authority = match (hosts.next(), hosts.next()) {
        (Some(host), None) => Some(host.to_str().ok()?.parse::<Authority>().ok()?),
        (None, None) => None,
        _ => return None,
    };
    let uri_authority = request.uri().authority();
    if let (Some(header), Some(uri)) = (&header_authority, uri_authority)
        && !header.as_str().eq_ignore_ascii_case(uri.as_str())
    {
        return None;
    }
    let authority = header_authority.or_else(|| uri_authority.cloned())?;
    if !valid_authority(&authority) {
        return None;
    }
    let host = authority.host();
    let ip = host.trim_start_matches('[').trim_end_matches(']');
    (host.eq_ignore_ascii_case("localhost")
        || ip
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback()))
    .then_some(authority)
}

fn valid_authority(authority: &Authority) -> bool {
    !authority.as_str().contains('@')
        && authority
            .as_str()
            .strip_prefix(authority.host())
            .is_some_and(|suffix| {
                suffix.is_empty()
                    || suffix
                        .strip_prefix(':')
                        .is_some_and(|port| port.parse::<u16>().is_ok())
            })
}

fn same_origin(origin: &Uri, local: &Authority) -> bool {
    let Some(scheme @ ("http" | "https")) = origin.scheme_str() else {
        return false;
    };
    if origin.query().is_some() || !matches!(origin.path(), "" | "/") {
        return false;
    }
    let Some(authority) = origin.authority().filter(|value| valid_authority(value)) else {
        return false;
    };
    let default_port = if scheme == "https" { 443 } else { 80 };
    authority.host().eq_ignore_ascii_case(local.host())
        && authority.port_u16().unwrap_or(default_port) == local.port_u16().unwrap_or(default_port)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;

    #[test]
    fn local_authorities_and_origins_are_exact_and_unambiguous() {
        for (host, origin, expected) in [
            ("localhost:3000", "http://localhost:3000", true),
            ("[::1]:3000", "http://[::1]:3000", true),
            ("localhost:80", "http://localhost", true),
            (
                "localhost.evil.example",
                "http://localhost.evil.example",
                false,
            ),
            ("localhost", "http://localhost/path", false),
            ("localhost", "http://localhost?query=1", false),
            ("localhost", "http://user@localhost", false),
            ("localhost:99999", "http://localhost:99999", false),
        ] {
            let request = Request::post("/")
                .header(header::HOST, host)
                .header(header::ORIGIN, origin)
                .body(Body::empty())
                .unwrap();
            assert_eq!(allows(&request), expected, "{host} {origin}");
        }
        let duplicate_host = Request::get("/")
            .header(header::HOST, "localhost")
            .header(header::HOST, "attacker.example")
            .body(Body::empty())
            .unwrap();
        assert!(!allows(&duplicate_host));
        let duplicate_origin = Request::post("/")
            .header(header::HOST, "localhost")
            .header(header::ORIGIN, "http://localhost")
            .header(header::ORIGIN, "https://attacker.example")
            .body(Body::empty())
            .unwrap();
        assert!(!allows(&duplicate_origin));
    }
}
