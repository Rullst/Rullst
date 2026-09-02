use crate::ai::AiError;
use std::net::IpAddr;

const MAX_BASE_URL_BYTES: usize = 2_048;
const MAX_MODEL_BYTES: usize = 256;
const MAX_API_KEY_BYTES: usize = 8 * 1_024;

#[derive(Clone, Copy)]
pub(super) enum EndpointScope {
    Loopback,
    Cloud,
}

pub(super) fn validate_base_url(base_url: String, scope: EndpointScope) -> Result<String, AiError> {
    if base_url.is_empty()
        || base_url.len() > MAX_BASE_URL_BYTES
        || base_url.trim() != base_url
        || base_url.chars().any(char::is_control)
    {
        return Err(configuration_error(
            "base URL is empty, oversized, or malformed",
        ));
    }
    let parsed = reqwest::Url::parse(&base_url)
        .map_err(|_| configuration_error("base URL is not a valid absolute URL"))?;
    if !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err(configuration_error(
            "base URL cannot contain credentials, query, or fragment",
        ));
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| configuration_error("base URL requires a host"))?;
    match scope {
        EndpointScope::Loopback => {
            let host = host
                .strip_prefix('[')
                .and_then(|host| host.strip_suffix(']'))
                .unwrap_or(host);
            let loopback = host
                .parse::<IpAddr>()
                .is_ok_and(|address| address.is_loopback());
            if !loopback || !matches!(parsed.scheme(), "http" | "https") {
                return Err(configuration_error(
                    "local base URL must use HTTP(S) with a literal loopback IP",
                ));
            }
        }
        EndpointScope::Cloud if parsed.scheme() != "https" => {
            return Err(configuration_error("cloud base URL must use HTTPS"));
        }
        EndpointScope::Cloud => {}
    }
    Ok(parsed.as_str().trim_end_matches('/').to_string())
}

pub(super) fn validate_model(field: &str, model: &str) -> Result<(), AiError> {
    if model.is_empty()
        || model.len() > MAX_MODEL_BYTES
        || model.trim() != model
        || model.chars().any(char::is_control)
    {
        return Err(configuration_error(&format!("{field} is invalid")));
    }
    Ok(())
}

pub(super) fn validate_api_key(api_key: &str) -> Result<(), AiError> {
    if api_key.len() > MAX_API_KEY_BYTES
        || !api_key.is_ascii()
        || api_key
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
    {
        return Err(configuration_error("API key is oversized or malformed"));
    }
    Ok(())
}

fn configuration_error(message: &str) -> AiError {
    AiError::ConfigError(format!("OpenAI-compatible {message}"))
}
