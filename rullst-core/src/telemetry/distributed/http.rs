use super::{Counters, TelemetryConfig, TelemetryError};
use opentelemetry_http::{Bytes, HttpClient, HttpError, Request, Response};
use opentelemetry_proto::tonic::collector::trace::v1::ExportTraceServiceResponse;
use prost::Message;
use std::{
    fmt,
    io::Read,
    sync::{
        Arc, Mutex, OnceLock, PoisonError,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

/// Largest request body sent to the collector.
const MAX_REQUEST_BYTES: usize = 1024 * 1024;
/// Minimum time between two stderr reports of failed legacy exports.
const FAILURE_REPORT_INTERVAL: Duration = Duration::from_secs(60);

/// This private client is polled only by the SDK's dedicated exporter thread.
/// Its lazy blocking client is created there, never on an async request task.
pub(in crate::telemetry) struct BoundedOtlpClient {
    endpoint: String,
    token: Option<String>,
    ca: Option<Vec<u8>>,
    timeout: Duration,
    client: OnceLock<Result<reqwest::blocking::Client, TelemetryError>>,
    counters: Arc<Counters>,
    /// The `init_telemetry` profile: it forwards SDK headers and honours the
    /// proxy environment variables, and since its counters are not observable
    /// it reports failed exports on stderr instead.
    legacy: bool,
    failures: FailureReport,
}

impl fmt::Debug for BoundedOtlpClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("BoundedOtlpClient([REDACTED])")
    }
}

impl BoundedOtlpClient {
    pub(super) fn new(
        config: &TelemetryConfig,
        counters: Arc<Counters>,
    ) -> Result<Self, TelemetryError> {
        Ok(Self {
            endpoint: config
                .endpoint
                .clone()
                .ok_or(TelemetryError::InvalidConfig)?,
            token: config.token.clone(),
            ca: config.ca.clone(),
            timeout: config.timeout,
            client: OnceLock::new(),
            counters,
            legacy: false,
            failures: FailureReport::default(),
        })
    }

    pub(in crate::telemetry) fn legacy(endpoint: String) -> Self {
        Self {
            endpoint,
            token: None,
            ca: None,
            timeout: Duration::from_secs(3),
            client: OnceLock::new(),
            counters: Arc::default(),
            legacy: true,
            failures: FailureReport::default(),
        }
    }

    fn send(&self, request: Request<Bytes>) -> Result<Response<Bytes>, TelemetryError> {
        if request.body().len() > MAX_REQUEST_BYTES
            || request.method() != http::Method::POST
            || request
                .headers()
                .get(http::header::CONTENT_TYPE)
                .is_none_or(|v| v != "application/x-protobuf")
            || request
                .headers()
                .contains_key(http::header::CONTENT_ENCODING)
        {
            return Err(TelemetryError::Export);
        }
        let client = self
            .client
            .get_or_init(|| {
                let mut builder = reqwest::blocking::Client::builder();
                if !self.legacy {
                    // The minimized profile never follows ambient proxies.
                    builder = builder.no_proxy();
                }
                let mut builder = builder
                    .tls_backend_rustls()
                    .redirect(reqwest::redirect::Policy::none())
                    .timeout(self.timeout)
                    .connect_timeout(self.timeout)
                    .pool_max_idle_per_host(1);
                if let Some(pem) = &self.ca {
                    builder = builder.add_root_certificate(
                        reqwest::Certificate::from_pem(pem)
                            .map_err(|_| TelemetryError::InvalidConfig)?,
                    );
                }
                builder.build().map_err(|_| TelemetryError::Export)
            })
            .as_ref()
            .map_err(|error| *error)?;
        // The explicit endpoint wins even if an ambient SDK header/configuration
        // changed. The minimized profile forwards no environmental headers.
        let mut outbound = client
            .post(&self.endpoint)
            .header(http::header::CONTENT_TYPE, "application/x-protobuf");
        if self.legacy {
            outbound = outbound.headers(request.headers().clone());
        }
        if let Some(token) = &self.token {
            outbound = outbound.bearer_auth(token);
        }
        let response = outbound
            .body(request.into_body())
            .send()
            .map_err(|_| TelemetryError::Export)?;
        if response.status() != http::StatusCode::OK
            || response.content_length().is_some_and(|size| size > 16384)
            || response
                .headers()
                .get(http::header::CONTENT_TYPE)
                .is_some_and(|value| value != "application/x-protobuf")
        {
            return Err(TelemetryError::Export);
        }
        let mut bytes = Vec::new();
        response
            .take(16385)
            .read_to_end(&mut bytes)
            .map_err(|_| TelemetryError::Export)?;
        if bytes.len() > 16384 {
            return Err(TelemetryError::Export);
        }
        let result = ExportTraceServiceResponse::decode(bytes.as_slice())
            .map_err(|_| TelemetryError::Export)?;
        if result
            .partial_success
            .is_some_and(|partial| partial.rejected_spans != 0 || !partial.error_message.is_empty())
        {
            // No retry: a partially accepted batch may already be stored. Do not
            // copy the collector's arbitrary diagnostic string into local logs.
            return Err(TelemetryError::Export);
        }
        Response::builder()
            .status(200)
            .body(Bytes::from(bytes))
            .map_err(|_| TelemetryError::Export)
    }
}

#[async_trait::async_trait]
impl HttpClient for BoundedOtlpClient {
    async fn send_bytes(&self, request: Request<Bytes>) -> Result<Response<Bytes>, HttpError> {
        let bytes = request.body().len();
        match self.send(request) {
            Ok(response) => {
                self.counters
                    .accepted_batches
                    .fetch_add(1, Ordering::Relaxed);
                Ok(response)
            }
            Err(error) => {
                self.counters.failed_batches.fetch_add(1, Ordering::Relaxed);
                if self.legacy {
                    self.failures.report(bytes, Instant::now());
                }
                Err(Box::new(error))
            }
        }
    }
}

/// Rate-limited stderr report of failed legacy exports. It never includes
/// the endpoint, span data or the collector's response.
#[derive(Default)]
struct FailureReport {
    last: Mutex<Option<Instant>>,
    suppressed: AtomicU64,
}

impl FailureReport {
    fn report(&self, bytes: usize, now: Instant) {
        let Some(suppressed) = self.admit(now) else {
            return;
        };
        let cause = if bytes > MAX_REQUEST_BYTES {
            format!("the {bytes}-byte batch exceeds the {MAX_REQUEST_BYTES}-byte request limit")
        } else {
            "the collector request failed or was rejected".to_string()
        };
        crate::server::console::stderr_line(format_args!(
            "Rullst telemetry: dropped a span batch because {cause} ({suppressed} more failed batches since the previous report)"
        ));
    }

    /// Returns the number of failures suppressed since the previous report
    /// when a report is due, counting this one as suppressed otherwise.
    fn admit(&self, now: Instant) -> Option<u64> {
        let mut last = self.last.lock().unwrap_or_else(PoisonError::into_inner);
        if last.is_some_and(|last| now.saturating_duration_since(last) < FAILURE_REPORT_INTERVAL) {
            self.suppressed.fetch_add(1, Ordering::Relaxed);
            return None;
        }
        *last = Some(now);
        Some(self.suppressed.swap(0, Ordering::Relaxed))
    }
}

#[cfg(test)]
#[test]
fn failure_reports_are_rate_limited_and_count_suppressed_batches() {
    let report = FailureReport::default();
    let start = Instant::now();
    assert_eq!(report.admit(start), Some(0));
    assert_eq!(report.admit(start + Duration::from_secs(1)), None);
    assert_eq!(report.admit(start + Duration::from_secs(59)), None);
    assert_eq!(report.admit(start + FAILURE_REPORT_INTERVAL), Some(2));
    assert_eq!(report.admit(start + FAILURE_REPORT_INTERVAL), None);
}
