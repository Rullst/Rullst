//! Explicit operation-label allowlist, minimized OTLP export and trusted parents.
mod config;
mod context;
mod http;
mod processor;
#[cfg(test)]
mod tests;
pub use config::TelemetryConfig;
pub use context::{ParentPolicy, TraceParent};
pub(in crate::telemetry) use http::BoundedOtlpClient;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::{WithExportConfig, WithHttpConfig};
use opentelemetry_sdk::{
    Resource,
    trace::{BatchConfigBuilder, BatchSpanProcessor, Sampler, SdkTracerProvider, SpanLimits},
};
use processor::{Destination, MinimizedProcessor};
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

/// Secret-minimized telemetry errors; no collector URL, token or error body.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum TelemetryError {
    /// An explicit configuration value violates the supported profile.
    #[error("invalid telemetry configuration")]
    InvalidConfig,
    /// A propagated context is malformed, duplicated or unsupported.
    #[error("invalid trace context")]
    InvalidContext,
    /// The requested subscriber/context or lifecycle is unavailable.
    #[error("telemetry unavailable")]
    Unavailable,
    /// Export failed, timed out or received a malformed/partial response.
    #[error("telemetry export failed")]
    Export,
}

#[derive(Debug, Default)]
pub(super) struct Counters {
    forwarded_spans: AtomicU64,
    rejected_spans: AtomicU64,
    offline_spans: AtomicU64,
    accepted_batches: AtomicU64,
    failed_batches: AtomicU64,
}

/// Process-local cumulative observations, not durable delivery receipts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TelemetryStatus {
    /// Approved spans offered to the SDK queue; saturation may still drop them.
    pub forwarded_spans: u64,
    /// Spans rejected by the operation/metadata contract before queueing.
    pub rejected_spans: u64,
    /// Spans explicitly discarded in the offline fixture without network I/O.
    pub offline_spans: u64,
    /// HTTP batches with a valid fully accepted OTLP response, not storage proof.
    pub accepted_batches: u64,
    /// HTTP export failures, including partial acceptance and malformed responses.
    pub failed_batches: u64,
}

/// Owned minimized SDK pipeline. Install its layer explicitly and shut it down
/// after dropping instrumented spans. No global subscriber/provider is replaced.
#[must_use = "retain the telemetry runtime and explicitly flush/shutdown it"]
pub struct DistributedTelemetry {
    provider: SdkTracerProvider,
    counters: Arc<Counters>,
    offline: bool,
    lifecycle: Arc<tokio::sync::Semaphore>,
}

impl DistributedTelemetry {
    /// Creates a bounded background exporter. Construction validates configuration;
    /// it does not contact or certify the collector.
    pub fn try_new(config: TelemetryConfig) -> Result<Self, TelemetryError> {
        let counters = Arc::new(Counters::default());
        let offline = config.endpoint.is_none();
        let exporter = if let Some(endpoint) = &config.endpoint {
            Destination::Remote(
                opentelemetry_otlp::SpanExporter::builder()
                    .with_http()
                    .with_endpoint(endpoint)
                    .with_protocol(opentelemetry_otlp::Protocol::HttpBinary)
                    .with_timeout(config.timeout)
                    .with_http_client(BoundedOtlpClient::new(&config, counters.clone())?)
                    .build()
                    .map_err(|_| TelemetryError::InvalidConfig)?,
            )
        } else {
            Destination::Offline(counters.clone())
        };
        let batch = BatchSpanProcessor::builder(exporter)
            .with_batch_config(
                BatchConfigBuilder::default()
                    .with_max_queue_size(config.queue)
                    .with_max_export_batch_size(config.batch)
                    .with_scheduled_delay(config.interval)
                    .build(),
            )
            .build();
        let processor = MinimizedProcessor {
            inner: batch,
            operations: config.operations,
            counters: counters.clone(),
        };
        let provider = SdkTracerProvider::builder()
            .with_span_processor(processor)
            .with_resource(
                Resource::builder_empty()
                    .with_service_name(config.service)
                    .build(),
            )
            .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
                config.ratio,
            ))))
            .with_span_limits(SpanLimits {
                max_events_per_span: 0,
                max_attributes_per_span: 0,
                max_links_per_span: 0,
                max_attributes_per_event: 0,
                max_attributes_per_link: 0,
            })
            .build();
        Ok(Self {
            provider,
            counters,
            offline,
            lifecycle: Arc::new(tokio::sync::Semaphore::new(1)),
        })
    }

    /// Creates a composable tracing layer. Other subscriber layers have their own
    /// metadata policy; this profile cannot redact their independently emitted data.
    pub fn layer<S>(
        &self,
    ) -> tracing_opentelemetry::OpenTelemetryLayer<S, opentelemetry_sdk::trace::SdkTracer>
    where
        S: tracing::Subscriber + for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>,
    {
        tracing_opentelemetry::layer().with_tracer(self.provider.tracer("rullst.minimized"))
    }

    /// Identifies explicit offline operation; remote errors never enable this mode.
    pub fn is_mock(&self) -> bool {
        self.offline
    }

    /// Returns bounded metadata-only diagnostics without URLs or trace values.
    pub fn status(&self) -> TelemetryStatus {
        TelemetryStatus {
            forwarded_spans: self.counters.forwarded_spans.load(Ordering::Relaxed),
            rejected_spans: self.counters.rejected_spans.load(Ordering::Relaxed),
            offline_spans: self.counters.offline_spans.load(Ordering::Relaxed),
            accepted_batches: self.counters.accepted_batches.load(Ordering::Relaxed),
            failed_batches: self.counters.failed_batches.load(Ordering::Relaxed),
        }
    }

    /// Flushes queued spans on a blocking worker. Prior dropped/failed exports
    /// are not recovered by a successful flush; inspect status and collector health.
    pub async fn flush(&self) -> Result<(), TelemetryError> {
        let permit = self
            .lifecycle
            .clone()
            .try_acquire_owned()
            .map_err(|_| TelemetryError::Unavailable)?;
        let runtime =
            tokio::runtime::Handle::try_current().map_err(|_| TelemetryError::Unavailable)?;
        let provider = self.provider.clone();
        runtime
            .spawn_blocking(move || {
                let _permit = permit;
                provider.force_flush()
            })
            .await
            .map_err(|_| TelemetryError::Unavailable)?
            .map_err(|_| TelemetryError::Export)
    }

    /// Closes this pipeline with an SDK five-second shutdown deadline. Drop active
    /// spans first. A timeout/error does not prove queued telemetry was delivered.
    pub async fn shutdown(self) -> Result<TelemetryStatus, TelemetryError> {
        let status = self.counters.clone();
        let runtime =
            tokio::runtime::Handle::try_current().map_err(|_| TelemetryError::Unavailable)?;
        runtime
            .spawn_blocking(move || self.provider.shutdown_with_timeout(Duration::from_secs(5)))
            .await
            .map_err(|_| TelemetryError::Unavailable)?
            .map_err(|_| TelemetryError::Export)?;
        Ok(TelemetryStatus {
            forwarded_spans: status.forwarded_spans.load(Ordering::Relaxed),
            rejected_spans: status.rejected_spans.load(Ordering::Relaxed),
            offline_spans: status.offline_spans.load(Ordering::Relaxed),
            accepted_batches: status.accepted_batches.load(Ordering::Relaxed),
            failed_batches: status.failed_batches.load(Ordering::Relaxed),
        })
    }
}
