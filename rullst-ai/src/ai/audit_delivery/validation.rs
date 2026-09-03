use super::AuditDeliveryError;
use std::net::IpAddr;

const MAX_ENDPOINT_BYTES: usize = 2_048;
const MAX_IDENTIFIER_BYTES: usize = 128;
const MIN_LIVE_KEY_BYTES: usize = 32;
const MAX_KEY_BYTES: usize = 4 * 1_024;

#[derive(Clone, Copy)]
pub(super) enum EndpointScope {
    Cloud,
    Loopback,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ClientMode {
    Live,
    Mock,
}

pub(super) fn validate_identifier(
    field: &'static str,
    value: &str,
) -> Result<(), AuditDeliveryError> {
    if value.is_empty()
        || value.len() > MAX_IDENTIFIER_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(AuditDeliveryError::InvalidConfiguration(match field {
            "source" => "audit source is invalid",
            "key ID" => "audit key ID is invalid",
            _ => "audit event ID is invalid",
        }));
    }
    Ok(())
}

pub(super) fn validate_endpoint(
    endpoint: String,
    scope: EndpointScope,
) -> Result<String, AuditDeliveryError> {
    if endpoint.is_empty()
        || endpoint.len() > MAX_ENDPOINT_BYTES
        || endpoint.trim() != endpoint
        || endpoint.chars().any(char::is_control)
    {
        return Err(AuditDeliveryError::InvalidConfiguration(
            "audit endpoint is empty, oversized, or malformed",
        ));
    }
    let parsed = reqwest::Url::parse(&endpoint).map_err(|_| {
        AuditDeliveryError::InvalidConfiguration("audit endpoint is not an absolute URL")
    })?;
    if !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err(AuditDeliveryError::InvalidConfiguration(
            "audit endpoint cannot contain credentials, query, or fragment",
        ));
    }
    let host = parsed
        .host_str()
        .ok_or(AuditDeliveryError::InvalidConfiguration(
            "audit endpoint requires a host",
        ))?;
    match scope {
        EndpointScope::Cloud if parsed.scheme() != "https" => {
            return Err(AuditDeliveryError::InvalidConfiguration(
                "remote audit endpoint must use HTTPS",
            ));
        }
        EndpointScope::Loopback => {
            let host = host
                .strip_prefix('[')
                .and_then(|value| value.strip_suffix(']'))
                .unwrap_or(host);
            if !matches!(parsed.scheme(), "http" | "https")
                || !host
                    .parse::<IpAddr>()
                    .is_ok_and(|address| address.is_loopback())
            {
                return Err(AuditDeliveryError::InvalidConfiguration(
                    "local audit endpoint must use HTTP(S) and a literal loopback IP",
                ));
            }
        }
        EndpointScope::Cloud => {}
    }
    Ok(parsed.to_string())
}

pub(super) fn validate_key(key: &str, mode: ClientMode) -> Result<(), AuditDeliveryError> {
    if key.len() > MAX_KEY_BYTES
        || !key.is_ascii()
        || key.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err(AuditDeliveryError::InvalidConfiguration(
            "audit signing key is oversized or malformed",
        ));
    }
    if mode == ClientMode::Live && key.len() < MIN_LIVE_KEY_BYTES {
        return Err(AuditDeliveryError::InvalidConfiguration(
            "live audit signing key must contain at least 32 bytes",
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "validation_tests.rs"]
mod validation_tests;
