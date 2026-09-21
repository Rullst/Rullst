use super::TelemetryError;
use std::{collections::BTreeSet, fmt, net::IpAddr, time::Duration};

/// Explicit collector, approved operation labels and bounded export policy.
#[derive(Clone)]
pub struct TelemetryConfig {
    pub(super) service: String,
    pub(super) endpoint: Option<String>,
    pub(super) operations: BTreeSet<String>,
    pub(super) token: Option<String>,
    pub(super) ca: Option<Vec<u8>>,
    pub(super) queue: usize,
    pub(super) batch: usize,
    pub(super) interval: Duration,
    pub(super) timeout: Duration,
    pub(super) ratio: f64,
}

impl TelemetryConfig {
    /// Uses verified HTTPS to an exact `/v1/traces` endpoint. Empty or `mock_*`
    /// endpoints select explicit offline discard/observation without networking.
    pub fn try_new(
        service: impl Into<String>,
        endpoint: impl Into<String>,
        operations: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self, TelemetryError> {
        Self::build(service.into(), endpoint.into(), operations, false)
    }

    /// Allows HTTP only for a literal loopback collector; never inferred from input.
    pub fn loopback_for_tests(
        service: impl Into<String>,
        endpoint: impl Into<String>,
        operations: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self, TelemetryError> {
        Self::build(service.into(), endpoint.into(), operations, true)
    }

    fn build(
        service: String,
        endpoint: String,
        operations: impl IntoIterator<Item = impl Into<String>>,
        loopback: bool,
    ) -> Result<Self, TelemetryError> {
        label(&service)?;
        let mut approved = BTreeSet::new();
        for operation in operations {
            let operation = operation.into();
            label(&operation)?;
            if approved.len() >= 64 || !approved.insert(operation) {
                return Err(TelemetryError::InvalidConfig);
            }
        }
        if approved.is_empty() {
            return Err(TelemetryError::InvalidConfig);
        }
        let endpoint = if endpoint.is_empty() || endpoint.starts_with("mock_") {
            None
        } else {
            if endpoint.len() > 2048 {
                return Err(TelemetryError::InvalidConfig);
            }
            let url = reqwest::Url::parse(&endpoint).map_err(|_| TelemetryError::InvalidConfig)?;
            let local = url
                .host_str()
                .and_then(|host| host.trim_matches(['[', ']']).parse::<IpAddr>().ok())
                .is_some_and(|ip| ip.is_loopback());
            if !(url.scheme() == "https" || loopback && url.scheme() == "http" && local)
                || !url.username().is_empty()
                || url.password().is_some()
                || url.host_str().is_none()
                || url.port() == Some(0)
                || url.query().is_some()
                || url.fragment().is_some()
                || url.path() != "/v1/traces"
            {
                return Err(TelemetryError::InvalidConfig);
            }
            Some(url.to_string())
        };
        Ok(Self {
            service,
            endpoint,
            operations: approved,
            token: None,
            ca: None,
            queue: 256,
            batch: 32,
            interval: Duration::from_secs(1),
            timeout: Duration::from_secs(3),
            ratio: 1.0,
        })
    }

    /// Sets 1–4,096 queued spans, 1–128 per batch (not above queue), a 100 ms–30 s
    /// interval and a 100 ms–10 s total network timeout. These override SDK env defaults.
    pub fn with_limits(
        mut self,
        queue: usize,
        batch: usize,
        interval: Duration,
        timeout: Duration,
    ) -> Result<Self, TelemetryError> {
        if !(1..=4096).contains(&queue)
            || !(1..=128).contains(&batch)
            || batch > queue
            || !(Duration::from_millis(100)..=Duration::from_secs(30)).contains(&interval)
            || !(Duration::from_millis(100)..=Duration::from_secs(10)).contains(&timeout)
        {
            return Err(TelemetryError::InvalidConfig);
        }
        self.queue = queue;
        self.batch = batch;
        self.interval = interval;
        self.timeout = timeout;
        Ok(self)
    }

    /// Samples roots at a finite ratio from zero to one. Trusted parents retain
    /// their decision; untrusted ingress must start a new trace.
    pub fn with_sampling(mut self, ratio: f64) -> Result<Self, TelemetryError> {
        if !ratio.is_finite() || !(0.0..=1.0).contains(&ratio) {
            return Err(TelemetryError::InvalidConfig);
        }
        self.ratio = ratio;
        Ok(self)
    }

    /// Supplies an explicit bearer token, without ambient OTLP headers. Transport
    /// diagnostics never include it. Secret custody/rotation belongs to the host.
    pub fn with_bearer_token(mut self, token: impl Into<String>) -> Result<Self, TelemetryError> {
        let token = token.into();
        if token.is_empty() || token.len() > 8192 || !token.bytes().all(|b| b.is_ascii_graphic()) {
            return Err(TelemetryError::InvalidConfig);
        }
        self.token = Some(token);
        Ok(self)
    }

    /// Adds at most 64 KiB of explicit PEM CA certificates, retaining hostname verification.
    pub fn with_ca_certificate(mut self, pem: impl Into<Vec<u8>>) -> Result<Self, TelemetryError> {
        let pem = pem.into();
        if pem.is_empty() || pem.len() > 65536 {
            return Err(TelemetryError::InvalidConfig);
        }
        let certificates = reqwest::Certificate::from_pem_bundle(&pem)
            .map_err(|_| TelemetryError::InvalidConfig)?;
        if certificates.is_empty() {
            return Err(TelemetryError::InvalidConfig);
        }
        let mut check = reqwest::Client::builder().no_proxy().tls_backend_rustls();
        for certificate in certificates {
            check = check.add_root_certificate(certificate);
        }
        let _validated = check.build().map_err(|_| TelemetryError::InvalidConfig)?;
        self.ca = Some(pem);
        Ok(self)
    }

    /// Rejects offline and HTTP profiles before a production deployment.
    pub fn require_production(self) -> Result<Self, TelemetryError> {
        if !self
            .endpoint
            .as_ref()
            .is_some_and(|value| value.starts_with("https://"))
        {
            return Err(TelemetryError::InvalidConfig);
        }
        Ok(self)
    }
}

impl fmt::Debug for TelemetryConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TelemetryConfig")
            .field("offline", &self.endpoint.is_none())
            .field("approved_operations", &self.operations.len())
            .field("queue", &self.queue)
            .field("batch", &self.batch)
            .finish_non_exhaustive()
    }
}

fn label(value: &str) -> Result<(), TelemetryError> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
    {
        return Err(TelemetryError::InvalidConfig);
    }
    Ok(())
}
