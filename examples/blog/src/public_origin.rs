//! Public-origin handling for links emitted by the deployable showcase.

use axum::http::Uri;

const PUBLIC_ORIGIN_ENV: &str = "RULLST_PUBLIC_ORIGIN";

/// Returns the configured canonical HTTPS origin without a trailing slash.
pub(crate) fn configured_public_origin() -> Option<String> {
    let configured = std::env::var(PUBLIC_ORIGIN_ENV).ok()?;
    normalize_public_origin(&configured)
}

/// Returns an origin suitable for deterministic offline fixture URLs.
pub(crate) fn fixture_origin() -> String {
    configured_public_origin().unwrap_or_else(|| {
        if cfg!(debug_assertions) {
            "http://127.0.0.1:3000".to_string()
        } else {
            "https://example.invalid".to_string()
        }
    })
}

fn normalize_public_origin(candidate: &str) -> Option<String> {
    let uri = candidate.trim().parse::<Uri>().ok()?;
    if uri.scheme_str() != Some("https") {
        return None;
    }

    let authority = uri.authority()?.as_str();
    if authority.contains('@') {
        return None;
    }

    let path_and_query = uri
        .path_and_query()
        .map(|value| value.as_str())
        .unwrap_or("/");
    if path_and_query != "/" {
        return None;
    }

    Some(format!("https://{authority}"))
}

#[cfg(test)]
mod tests {
    use super::normalize_public_origin;

    #[test]
    fn public_origin_accepts_only_an_https_origin() {
        assert_eq!(
            normalize_public_origin("https://showcase.example:8443/"),
            Some("https://showcase.example:8443".to_string())
        );
        assert_eq!(normalize_public_origin("http://showcase.example"), None);
        assert_eq!(
            normalize_public_origin("https://user@showcase.example"),
            None
        );
        assert_eq!(
            normalize_public_origin("https://showcase.example/path"),
            None
        );
        assert_eq!(normalize_public_origin("not a URI"), None);
    }
}
