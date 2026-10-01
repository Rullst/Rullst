//! Browser-origin boundary for the explicitly enabled local development CMS.

use axum::{
    extract::Request,
    http::{HeaderMap, HeaderValue, Method, Uri, header, uri::Authority},
};

/// Referrer policy stamped on responses of the local policy when the
/// application sets none.
///
/// Browsers derive the `Origin` of a same-origin POST or DELETE from the
/// document's policy: under `no-referrer` they send `Origin: null`, which
/// [`allows`] can only accept with Fetch Metadata. `same-origin` keeps the
/// real origin on Nexus's own requests and still sends no referrer elsewhere.
pub(super) const LOCAL_REFERRER_POLICY: &str = "same-origin";

pub(super) fn allows(request: &Request) -> bool {
    let Some(authority) = local_authority(request) else {
        return false;
    };
    let mut origins = request.headers().get_all(header::ORIGIN).iter();
    match (origins.next(), origins.next()) {
        (Some(origin), None) => {
            origin
                .to_str()
                .ok()
                .and_then(|origin| origin.parse::<Uri>().ok())
                .is_some_and(|origin| same_origin(&origin, &authority))
                || opaque_origin_from_same_origin_document(origin, request.headers())
        }
        (None, None) => matches!(
            *request.method(),
            Method::GET | Method::HEAD | Method::OPTIONS
        ),
        _ => false,
    }
}

/// Accepts `Origin: null` only when Fetch Metadata proves that a document of
/// this exact origin sent the request, as browsers do for Nexus's own fetch
/// and form requests when a host layer imposes `Referrer-Policy: no-referrer`.
/// `Sec-Fetch-Site` is a forbidden request header that page scripts cannot
/// set; other origins, including another local port (`same-site`), and a
/// bare or duplicated signal stay rejected.
fn opaque_origin_from_same_origin_document(origin: &HeaderValue, headers: &HeaderMap) -> bool {
    let mut sites = headers.get_all("sec-fetch-site").iter();
    origin.as_bytes() == b"null"
        && matches!(
            (sites.next(), sites.next()),
            (Some(site), None) if site.as_bytes() == b"same-origin"
        )
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
            .is_ok_and(|ip| ip.to_canonical().is_loopback()))
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
            (
                "[::ffff:127.0.0.1]:3000",
                "http://[::ffff:127.0.0.1]:3000",
                true,
            ),
            (
                "[::ffff:192.0.2.1]:3000",
                "http://[::ffff:192.0.2.1]:3000",
                false,
            ),
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

    #[test]
    fn opaque_origins_need_same_origin_fetch_metadata() {
        let request = |method: Method, origin: Option<&str>, sites: &[&str]| {
            let mut builder = Request::builder()
                .method(method)
                .uri("/")
                .header(header::HOST, "127.0.0.1:3000");
            if let Some(origin) = origin {
                builder = builder.header(header::ORIGIN, origin);
            }
            for site in sites {
                builder = builder.header("sec-fetch-site", *site);
            }
            builder.body(Body::empty()).unwrap()
        };
        // Nexus's own fetch POST/DELETE under `Referrer-Policy: no-referrer`.
        for method in [Method::POST, Method::DELETE, Method::PUT] {
            assert!(allows(&request(method, Some("null"), &["same-origin"])));
        }
        for (origin, sites) in [
            // A bare opaque origin carries no proof.
            (Some("null"), &[][..]),
            (Some("null"), &["same-site"][..]),
            (Some("null"), &["cross-site"][..]),
            (Some("null"), &["none"][..]),
            (Some("null"), &["same-origin", "cross-site"][..]),
            (Some("null"), &["same-origin", "same-origin"][..]),
            (Some("NULL"), &["same-origin"][..]),
            // Fetch Metadata never overrides a present foreign origin.
            (Some("http://attacker.example"), &["same-origin"][..]),
            (Some("http://127.0.0.1:3001"), &["same-origin"][..]),
            // No origin at all: an unsafe method still fails closed.
            (None, &["same-origin"][..]),
        ] {
            assert!(
                !allows(&request(Method::POST, origin, sites)),
                "{origin:?} {sites:?}"
            );
        }
        // A DNS-rebinding Host is rejected before any origin signal counts.
        let mut rebinding = request(Method::POST, Some("null"), &["same-origin"]);
        rebinding
            .headers_mut()
            .insert(header::HOST, "attacker.example".parse().unwrap());
        assert!(!allows(&rebinding));
    }

    #[tokio::test]
    async fn local_policy_serves_a_referrer_policy_that_keeps_real_origins() {
        use axum::{Router, extract::ConnectInfo, middleware, routing::get};
        use tower::ServiceExt;

        let app = Router::new()
            .route("/page", get(|| async { "page" }).post(|| async { "saved" }))
            .route(
                "/strict",
                get(|| async { ([(header::REFERRER_POLICY, "no-referrer")], "strict") }),
            )
            .layer(middleware::from_fn(super::super::loopback_only_middleware));
        let send = |method: Method, path: &str, origin: Option<&str>, site: Option<&str>| {
            let mut builder = Request::builder()
                .method(method)
                .uri(path)
                .header(header::HOST, "127.0.0.1:3000");
            if let Some(origin) = origin {
                builder = builder.header(header::ORIGIN, origin);
            }
            if let Some(site) = site {
                builder = builder.header("sec-fetch-site", site);
            }
            let mut request = builder.body(Body::empty()).unwrap();
            request.extensions_mut().insert(ConnectInfo(
                "127.0.0.1:41000".parse::<std::net::SocketAddr>().unwrap(),
            ));
            app.clone().oneshot(request)
        };

        let page = send(Method::GET, "/page", None, None).await.unwrap();
        assert_eq!(page.status(), axum::http::StatusCode::OK);
        assert_eq!(
            page.headers().get(header::REFERRER_POLICY).unwrap(),
            LOCAL_REFERRER_POLICY
        );
        // An application's own stricter policy is kept.
        let strict = send(Method::GET, "/strict", None, None).await.unwrap();
        assert_eq!(
            strict.headers().get(header::REFERRER_POLICY).unwrap(),
            "no-referrer"
        );

        let saved = send(Method::POST, "/page", Some("null"), Some("same-origin"))
            .await
            .unwrap();
        assert_eq!(saved.status(), axum::http::StatusCode::OK);
        for site in [None, Some("same-site"), Some("cross-site")] {
            let denied = send(Method::POST, "/page", Some("null"), site)
                .await
                .unwrap();
            assert_eq!(
                denied.status(),
                axum::http::StatusCode::FORBIDDEN,
                "{site:?}"
            );
            assert!(denied.headers().get(header::REFERRER_POLICY).is_none());
        }
    }
}
