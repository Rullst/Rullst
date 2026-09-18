//! Application-owned action links and body-only, URL-aware secret redaction.

use crate::{MailError, security::redact_email_secrets};

/// A validated link to an application-owned account action.
///
/// The origin must come from server configuration, never from a Host header or
/// browser input. Token issuance, expiry and single-use consumption belong to Auth.
#[derive(Clone, PartialEq, Eq)]
pub struct ActionLink(String);

impl std::fmt::Debug for ActionLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ActionLink([REDACTED])")
    }
}

impl ActionLink {
    /// Validates HTTPS and an exact configured origin. Local HTTP is accepted
    /// only for localhost or a literal loopback IP, on the configured port.
    pub fn new(
        url: impl Into<String>,
        application_origin: impl Into<String>,
    ) -> Result<Self, MailError> {
        let url = url.into();
        let parsed = safe_url(&url).ok_or_else(invalid_link)?;
        let origin = safe_url(&application_origin.into()).ok_or_else(invalid_link)?;
        if parsed.origin() != origin.origin()
            || origin.path() != "/"
            || origin.query().is_some()
            || origin.fragment().is_some()
        {
            return Err(invalid_link());
        }
        // An action link is an intentional token destination, never a carrier
        // for passwords, provider keys or authorization headers.
        if redact_body_secrets(&url) != url {
            return Err(invalid_link());
        }
        Ok(Self(url))
    }

    /// Exposes the sensitive URL only for delivery to its intended recipient.
    pub fn expose_url(&self) -> &str {
        &self.0
    }
}

fn invalid_link() -> MailError {
    MailError::ValidationError(
        "action link must use the configured secure origin and contain no accidental credentials"
            .into(),
    )
}

fn safe_url(value: &str) -> Option<reqwest::Url> {
    if value.len() > 4096 || value.chars().any(char::is_control) {
        return None;
    }
    let url = reqwest::Url::parse(value).ok()?;
    let host = url.host_str()?;
    let local = host == "localhost"
        || host
            .trim_matches(['[', ']'])
            .parse::<std::net::IpAddr>()
            .is_ok_and(|address| address.is_loopback());
    if !(url.scheme() == "https" || (url.scheme() == "http" && local))
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    Some(url)
}

/// Keeps opaque `token` query values only inside structurally safe action URLs.
/// Subjects, errors and telemetry must continue using the strict redactor.
pub(crate) fn redact_body_secrets(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut remaining = input;
    while let Some(start) = remaining
        .find("https://")
        .into_iter()
        .chain(remaining.find("http://"))
        .min()
    {
        let tail = &remaining[start..];
        let end = tail
            .find(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>'))
            .unwrap_or(tail.len());
        let candidate = &tail[..end];
        if remaining[..start]
            .chars()
            .next_back()
            .is_some_and(|c| !c.is_whitespace() && !matches!(c, '"' | '\'' | '>' | '('))
        {
            output.push_str(&redact_email_secrets(&remaining[..start + end]));
            remaining = &tail[end..];
            continue;
        }
        output.push_str(&redact_email_secrets(&remaining[..start]));
        if safe_url(candidate).is_some() {
            output.push_str(&redact_url(candidate));
        } else {
            output.push_str(&redact_email_secrets(candidate));
        }
        remaining = &tail[end..];
    }
    output.push_str(&redact_email_secrets(remaining));
    output
}

fn redact_url(url: &str) -> String {
    let Some((base, query)) = url.split_once('?') else {
        return redact_email_secrets(url);
    };
    let (query, fragment) = query
        .split_once('#')
        .map_or((query, None), |(q, f)| (q, Some(f)));
    let mut result = redact_email_secrets(base);
    result.push('?');
    for (index, parameter) in query.split('&').enumerate() {
        if index > 0 {
            result.push('&');
        }
        let plain = parameter.strip_prefix("amp;").unwrap_or(parameter);
        let opaque = plain.strip_prefix("token=").is_some_and(|token| {
            !token.is_empty()
                && token.len() <= 512
                && token
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~'))
                && redact_email_secrets(token) == token
        });
        if opaque {
            result.push_str(parameter);
        } else {
            result.push_str(&redact_email_secrets(parameter));
        }
    }
    if let Some(fragment) = fragment {
        result.push('#');
        result.push_str(&redact_email_secrets(fragment));
    }
    result
}
