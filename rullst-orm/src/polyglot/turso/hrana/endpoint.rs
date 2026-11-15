//! Pipeline URLs: the configured endpoint's, and the one a server names with
//! `base_url` for the rest of an interactive stream.

use reqwest::Url;

use super::{PolyglotError, protocol_error};

/// `<endpoint>/v3/pipeline`, with a `libsql://` endpoint sent over HTTPS.
pub(super) fn pipeline_url(endpoint: &str) -> Result<Url, PolyglotError> {
    let mut url = Url::parse(endpoint).map_err(|_| PolyglotError::InvalidConfiguration {
        backend: "Turso",
        reason: "endpoint is not a valid URL",
    })?;
    if url.scheme() == "libsql" {
        let suffix =
            url.as_str()
                .strip_prefix("libsql:")
                .ok_or(PolyglotError::InvalidConfiguration {
                    backend: "Turso",
                    reason: "libsql endpoint cannot be converted to HTTPS",
                })?;
        url = Url::parse(&format!("https:{suffix}")).map_err(|_| {
            PolyglotError::InvalidConfiguration {
                backend: "Turso",
                reason: "libsql endpoint cannot be converted to HTTPS",
            }
        })?;
    }
    with_pipeline_path(url).ok_or(PolyglotError::InvalidConfiguration {
        backend: "Turso",
        reason: "endpoint cannot be used as an HTTP base URL",
    })
}

/// The pipeline URL for the rest of a stream whose server answered with
/// `base_url`, as Hrana over HTTP lets a multi-instance server do.
///
/// It is accepted only at the configured pipeline URL's origin (scheme, host
/// and port, a default port matching its explicit form), so the bearer token
/// is never sent to another host; any other origin is
/// [`PolyglotError::CrossOriginEndpoint`]. The base URL must be absolute and
/// carry no credentials, query or fragment.
pub(super) fn continued_pipeline_url(
    configured: &Url,
    base_url: &str,
) -> Result<Url, PolyglotError> {
    let base =
        Url::parse(base_url).map_err(|_| protocol_error("base_url is not an absolute URL"))?;
    if base.origin() != configured.origin() {
        return Err(PolyglotError::CrossOriginEndpoint { backend: "Turso" });
    }
    if !base.username().is_empty()
        || base.password().is_some()
        || base.query().is_some()
        || base.fragment().is_some()
    {
        return Err(protocol_error(
            "base_url must not contain credentials, query parameters, or a fragment",
        ));
    }
    with_pipeline_path(base).ok_or_else(|| protocol_error("base_url cannot be used as a base URL"))
}

fn with_pipeline_path(mut url: Url) -> Option<Url> {
    let mut path = url.path_segments_mut().ok()?;
    path.pop_if_empty();
    path.push("v3").push("pipeline");
    drop(path);
    Some(url)
}
