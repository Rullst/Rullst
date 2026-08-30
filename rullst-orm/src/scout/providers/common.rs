use futures::StreamExt;
use reqwest::{Client, Response, StatusCode};
use serde_json::Value;
use url::Url;

use crate::Error;

pub const MAX_RESPONSE_BYTES: usize = 4 * 1_048_576;
pub const MAX_SEARCH_HITS: usize = 1_000;

pub fn mock_requested(values: &[&str]) -> bool {
    values.iter().any(|value| {
        let value = value.trim();
        value.is_empty()
            || value
                .get(..5)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case("mock_"))
            || value.eq_ignore_ascii_case("mock")
    })
}

pub fn validate_credential(field: &str, value: &str) -> Result<(), Error> {
    if value.len() > 2_048 || value.chars().any(char::is_control) {
        return Err(Error::Validation(format!(
            "Scout {field} credential exceeds its bound or contains control characters"
        )));
    }
    Ok(())
}

pub fn endpoint(value: &str) -> Result<Url, Error> {
    let url = Url::parse(value)
        .map_err(|_| Error::Validation("Scout endpoint must be an absolute URL".to_string()))?;
    let loopback = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "::1"));
    if url.scheme() != "https" && !(url.scheme() == "http" && loopback) {
        return Err(Error::Validation(
            "Scout endpoint must use HTTPS, except for an explicit loopback HTTP service"
                .to_string(),
        ));
    }
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !matches!(url.path(), "" | "/")
    {
        return Err(Error::Validation(
            "Scout endpoint must be an origin URL without credentials, path, query or fragment"
                .to_string(),
        ));
    }
    Ok(url)
}

pub fn client() -> Result<Client, Error> {
    Client::builder()
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("rullst-orm/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|_| Error::Internal("could not build Scout HTTP client".to_string()))
}

pub fn join(base: &Url, path: &str) -> Result<Url, Error> {
    base.join(path)
        .map_err(|_| Error::Internal("could not construct bounded Scout URL".to_string()))
}

pub async fn json_response(
    provider: &str,
    response: Response,
    accepted: &[StatusCode],
) -> Result<Value, Error> {
    if !accepted.contains(&response.status()) {
        return Err(Error::Internal(format!(
            "{provider} Scout request returned HTTP {}",
            response.status().as_u16()
        )));
    }
    let mut stream = response.bytes_stream();
    let mut body = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk
            .map_err(|_| Error::Internal(format!("{provider} Scout response could not be read")))?;
        if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            return Err(Error::Validation(format!(
                "{provider} Scout response exceeds {MAX_RESPONSE_BYTES} bytes"
            )));
        }
        body.extend_from_slice(&chunk);
    }
    if body.is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_slice(&body).map_err(Into::into)
}

pub fn request_error(provider: &str) -> Error {
    Error::Internal(format!("{provider} Scout transport failed"))
}

pub fn parse_positive_id(value: &Value, provider: &str) -> Result<i32, Error> {
    let id = if let Some(number) = value.as_i64() {
        i32::try_from(number).ok()
    } else {
        value.as_str().and_then(|text| text.parse::<i32>().ok())
    };
    id.filter(|id| *id > 0).ok_or_else(|| {
        Error::Validation(format!(
            "{provider} Scout response contains an invalid document id"
        ))
    })
}
