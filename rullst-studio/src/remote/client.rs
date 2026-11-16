//! Reads a project's monitoring document over the network, within bounds.
//!
//! The token goes only to the configured origin, as `Authorization: Bearer`,
//! on the fixed monitor paths: `rullst.monitor.v2` first, and
//! `rullst.monitor.v1` when the application answers `404` there (Rullst
//! before 13.0.0-alpha.6), which is then remembered for
//! [`LEGACY_RECHECK`]. Redirects are never followed, a proxy is used only as
//! the Settings page chooses ([`ProxyPlan`]), certificates are always
//! validated, and the response is limited in size and time. Errors are fixed statuses; the
//! remote body is never shown.

use super::document::{MONITOR_SCHEMA, MonitorDocument};
use super::document_v2::MONITOR_V2_SCHEMA;
use super::projects::Project;
use super::proxy::ProxyPlan;
use reqwest::{StatusCode, Url, header};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

/// How long a project that answered `404` for version 2 is read with
/// version 1 only, before version 2 is tried again.
pub(crate) const LEGACY_RECHECK: Duration = Duration::from_secs(600);
/// Projects remembered as version 1 only; the list is cleared when full.
const MAX_LEGACY_PROJECTS: usize = 64;

/// Size and time bounds for one read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct FetchLimits {
    /// Largest accepted response body.
    pub max_bytes: usize,
    /// Longest the whole request may take.
    pub timeout: Duration,
    /// Longest the connection may take.
    pub connect_timeout: Duration,
}

impl Default for FetchLimits {
    fn default() -> Self {
        Self {
            max_bytes: 1024 * 1024,
            timeout: Duration::from_secs(5),
            connect_timeout: Duration::from_secs(3),
        }
    }
}

/// Why a document could not be read; shown to the developer as a status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FetchError {
    /// The token cannot be decrypted; add the project again.
    TokenUnreadable,
    /// The base URL cannot form the monitor URL.
    InvalidUrl,
    /// No connection could be made (DNS, refused, TLS certificate).
    Unreachable,
    /// The project did not answer within the time limit.
    Timeout,
    /// The project answered with a redirect, which is never followed.
    Redirect,
    /// `401`: the token is not accepted.
    Unauthorized,
    /// `403`: refused (an authorizer denial or a browser-origin request).
    Forbidden,
    /// `404`: the monitor is not mounted, or the URL is wrong.
    NotMounted,
    /// `426`: the server requires TLS evidence.
    UpgradeRequired,
    /// `429`: rate limited or locked out.
    RateLimited,
    /// Another unexpected status.
    Status(u16),
    /// The body exceeded the size limit.
    TooLarge,
    /// The response is not JSON.
    NotJson,
    /// The body is not a valid `rullst.monitor.v1` or `rullst.monitor.v2`
    /// document.
    Invalid,
    /// The configured proxy cannot be used, so nothing is sent.
    ProxyUnusable,
}

impl FetchError {
    /// A fixed, human-readable status.
    pub fn message(&self) -> String {
        match self {
            Self::TokenUnreadable => {
                "the stored token cannot be decrypted; remove and add the project again".to_string()
            }
            Self::InvalidUrl => "the base URL cannot form a monitor URL".to_string(),
            Self::Unreachable => {
                "unreachable: connection, DNS or TLS certificate validation failed".to_string()
            }
            Self::Timeout => "no answer within the time limit".to_string(),
            Self::Redirect => {
                "the project answered with a redirect, which Studio never follows".to_string()
            }
            Self::Unauthorized => "401: the token is not accepted".to_string(),
            Self::Forbidden => "403: the request was refused".to_string(),
            Self::NotMounted => {
                "404: the monitor is not mounted (no credential configured) or the URL is wrong"
                    .to_string()
            }
            Self::UpgradeRequired => {
                "426: the server requires HTTPS or a trusted TLS proxy".to_string()
            }
            Self::RateLimited => "429: rate limited or locked out; wait and retry".to_string(),
            Self::Status(status) => format!("unexpected HTTP status {status}"),
            Self::TooLarge => "the response exceeded the size limit".to_string(),
            Self::NotJson => "the response is not JSON".to_string(),
            Self::Invalid => "the response is not a valid rullst.monitor document".to_string(),
            Self::ProxyUnusable => {
                "the proxy setting cannot be used; fix it on the Settings page".to_string()
            }
        }
    }
}

/// Monitor URLs that answered `404` for version 2, until when they are read
/// with version 1 only.
#[derive(Debug, Default)]
pub(crate) struct LegacyProjects {
    until: Mutex<HashMap<String, Instant>>,
}

impl LegacyProjects {
    /// Remembers `key` as version 1 only from `now`.
    pub(crate) fn mark_at(&self, key: &str, now: Instant) {
        let mut until = self.until.lock().unwrap_or_else(PoisonError::into_inner);
        if until.len() >= MAX_LEGACY_PROJECTS {
            until.clear();
        }
        until.insert(key.to_string(), now + LEGACY_RECHECK);
    }

    /// Whether `key` is read with version 1 only at `now`.
    pub(crate) fn is_legacy_at(&self, key: &str, now: Instant) -> bool {
        self.until
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(key)
            .is_some_and(|until| now < *until)
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.until
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }
}

/// Reads monitoring documents.
#[derive(Clone)]
pub struct MonitorClient {
    client: reqwest::Client,
    limits: FetchLimits,
    legacy: Arc<LegacyProjects>,
    /// Set when the proxy plan is [`ProxyPlan::Invalid`].
    blocked: bool,
}

impl MonitorClient {
    /// A direct client with `limits`.
    #[cfg(test)]
    pub(crate) fn new(limits: FetchLimits) -> Result<Self, FetchError> {
        Self::with_proxy(limits, &ProxyPlan::Direct)
    }

    /// A client with `limits` that connects as `plan` says. Fails only when
    /// the TLS stack cannot start.
    pub(crate) fn with_proxy(limits: FetchLimits, plan: &ProxyPlan) -> Result<Self, FetchError> {
        let builder = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(limits.timeout)
            .connect_timeout(limits.connect_timeout)
            .user_agent(concat!("rullst-studio/", env!("CARGO_PKG_VERSION")));
        let client = plan
            .apply(builder)
            .build()
            .map_err(|_| FetchError::Unreachable)?;
        Ok(Self {
            client,
            limits,
            legacy: Arc::default(),
            blocked: *plan == ProxyPlan::Invalid,
        })
    }

    #[cfg(test)]
    pub(crate) fn legacy(&self) -> &LegacyProjects {
        &self.legacy
    }

    /// Reads and validates `project`'s document: version 2, or version 1
    /// when the project does not serve version 2.
    pub async fn fetch(&self, project: &Project) -> Result<MonitorDocument, FetchError> {
        if self.blocked {
            return Err(FetchError::ProxyUnusable);
        }
        let token = project.token.as_ref().ok_or(FetchError::TokenUnreadable)?;
        let v1 = project.monitor_url().ok_or(FetchError::InvalidUrl)?;
        let v2 = project.monitor_v2_url().ok_or(FetchError::InvalidUrl)?;
        if !self.legacy.is_legacy_at(v1.as_str(), Instant::now()) {
            match self.read(v2, token.expose(), MONITOR_V2_SCHEMA).await {
                Err(FetchError::NotMounted) => self.legacy.mark_at(v1.as_str(), Instant::now()),
                outcome => return outcome,
            }
        }
        self.read(v1, token.expose(), MONITOR_SCHEMA).await
    }

    /// Reads the document at `url` and checks that it is `schema`.
    async fn read(
        &self,
        url: Url,
        token: &str,
        schema: &str,
    ) -> Result<MonitorDocument, FetchError> {
        let request = self
            .client
            .get(url)
            .bearer_auth(token)
            .header(header::ACCEPT, "application/json")
            .send();
        let body = tokio::time::timeout(self.limits.timeout, async {
            let response = request.await.map_err(classify)?;
            check_status(response.status())?;
            let json = response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| value.trim_start().starts_with("application/json"));
            if !json {
                return Err(FetchError::NotJson);
            }
            if response
                .content_length()
                .is_some_and(|length| length > self.limits.max_bytes as u64)
            {
                return Err(FetchError::TooLarge);
            }
            read_bounded(response, self.limits.max_bytes).await
        })
        .await
        .map_err(|_| FetchError::Timeout)??;
        let document: MonitorDocument =
            serde_json::from_slice(&body).map_err(|_| FetchError::Invalid)?;
        document.validate(schema).map_err(|_| FetchError::Invalid)?;
        Ok(document)
    }
}

fn check_status(status: StatusCode) -> Result<(), FetchError> {
    match status.as_u16() {
        200 => Ok(()),
        300..=399 => Err(FetchError::Redirect),
        401 => Err(FetchError::Unauthorized),
        403 => Err(FetchError::Forbidden),
        404 => Err(FetchError::NotMounted),
        426 => Err(FetchError::UpgradeRequired),
        429 => Err(FetchError::RateLimited),
        other => Err(FetchError::Status(other)),
    }
}

fn classify(error: reqwest::Error) -> FetchError {
    if error.is_timeout() {
        FetchError::Timeout
    } else {
        FetchError::Unreachable
    }
}

async fn read_bounded(
    mut response: reqwest::Response,
    max_bytes: usize,
) -> Result<Vec<u8>, FetchError> {
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(classify)? {
        if body.len().saturating_add(chunk.len()) > max_bytes {
            return Err(FetchError::TooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}
