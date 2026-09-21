use super::{Counters, TelemetryConfig, TelemetryError};
use opentelemetry_http::{Bytes, HttpClient, HttpError, Request, Response};
use opentelemetry_proto::tonic::collector::trace::v1::ExportTraceServiceResponse;
use prost::Message;
use std::{
    fmt,
    io::Read,
    sync::{Arc, OnceLock, atomic::Ordering},
    time::Duration,
};

/// This private client is polled only by the SDK's dedicated exporter thread.
/// Its lazy blocking client is created there, never on an async request task.
pub(in crate::telemetry) struct BoundedOtlpClient {
    endpoint: String,
    token: Option<String>,
    ca: Option<Vec<u8>>,
    timeout: Duration,
    client: OnceLock<Result<reqwest::blocking::Client, TelemetryError>>,
    counters: Arc<Counters>,
    legacy_headers: bool,
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
            legacy_headers: false,
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
            legacy_headers: true,
        }
    }

    fn send(&self, request: Request<Bytes>) -> Result<Response<Bytes>, TelemetryError> {
        if request.body().len() > 1024 * 1024
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
                let mut builder = reqwest::blocking::Client::builder()
                    .no_proxy()
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
        if self.legacy_headers {
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
        match self.send(request) {
            Ok(response) => {
                self.counters
                    .accepted_batches
                    .fetch_add(1, Ordering::Relaxed);
                Ok(response)
            }
            Err(error) => {
                self.counters.failed_batches.fetch_add(1, Ordering::Relaxed);
                Err(Box::new(error))
            }
        }
    }
}
