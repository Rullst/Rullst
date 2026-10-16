//! `Secure` on framework cookies sent over HTTPS, in every environment.
//!
//! Framework cookies (`rullst_session`, `rullst_csrf`, every cookie named
//! `rullst_*`) carry `Secure` in staging and production. A development or
//! test deployment reached over HTTPS, directly or through a trusted TLS
//! proxy, gets it too, so the attribute follows the transport rather than
//! only `RULLST_ENV`.

use axum::{
    extract::Request,
    http::{Extensions, HeaderMap, HeaderValue, Uri, header, uri::Scheme},
    middleware::Next,
    response::Response,
};

/// Whether the request reached the application over HTTPS: its URI has the
/// `https` scheme (HTTP/2 over TLS, or an absolute-form target), or
/// [`super::TrustedProxyLayer`] accepted `https` from a trusted proxy with
/// `trust_forwarded_proto` enabled. A forwarding header alone, from any
/// other peer, is never evidence.
pub(crate) fn request_is_https(uri: &Uri, extensions: &Extensions) -> bool {
    uri.scheme() == Some(&Scheme::HTTPS)
        || extensions
            .get::<super::ClientAddr>()
            .and_then(super::ClientAddr::forwarded_proto)
            .is_some_and(super::ForwardedProto::is_https)
}

/// Adds `Secure` to the response's `rullst_*` cookies that lack it when the
/// request arrived over HTTPS ([`request_is_https`]). Other cookies, and
/// every cookie of a plain-HTTP request, are left as they are; `Secure` is
/// never removed.
pub(crate) async fn secure_framework_cookies(req: Request, next: Next) -> Response {
    let https = request_is_https(req.uri(), req.extensions());
    let mut response = next.run(req).await;
    if https {
        add_secure_to_framework_cookies(response.headers_mut());
    }
    response
}

fn add_secure_to_framework_cookies(headers: &mut HeaderMap) {
    if !headers.get_all(header::SET_COOKIE).iter().any(needs_secure) {
        return;
    }
    let cookies = headers
        .get_all(header::SET_COOKIE)
        .iter()
        .map(|cookie| {
            if !needs_secure(cookie) {
                return cookie.clone();
            }
            let mut value = cookie.as_bytes().to_vec();
            value.extend_from_slice(b"; Secure");
            HeaderValue::from_bytes(&value).unwrap_or_else(|_| cookie.clone())
        })
        .collect::<Vec<_>>();
    headers.remove(header::SET_COOKIE);
    for cookie in cookies {
        headers.append(header::SET_COOKIE, cookie);
    }
}

/// Whether a `Set-Cookie` value names a `rullst_*` cookie without `Secure`.
fn needs_secure(cookie: &HeaderValue) -> bool {
    let mut parts = cookie.as_bytes().split(|byte| *byte == b';');
    let framework = parts
        .next()
        .is_some_and(|pair| pair.trim_ascii_start().starts_with(b"rullst_"));
    framework
        && !parts.any(|attribute| {
            let name = attribute
                .split(|byte| *byte == b'=')
                .next()
                .unwrap_or_default();
            name.trim_ascii().eq_ignore_ascii_case(b"secure")
        })
}

#[cfg(test)]
#[path = "secure_cookies_tests.rs"]
mod tests;
