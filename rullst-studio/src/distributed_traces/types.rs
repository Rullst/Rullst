use serde::{Deserialize, Serialize};

/// Wire version accepted by the distributed trace ingestion boundary.
pub const TRACE_BATCH_VERSION: u8 = 1;

/// A bounded, attribute-free batch of distributed trace spans.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceBatchV1 {
    /// Must equal [`TRACE_BATCH_VERSION`].
    pub version: u8,
    /// Spans submitted atomically after the complete batch is validated.
    pub spans: Vec<DistributedTraceSpanV1>,
}

impl TraceBatchV1 {
    /// Creates a v1 batch from caller-redacted spans.
    pub fn new(spans: Vec<DistributedTraceSpanV1>) -> Self {
        Self {
            version: TRACE_BATCH_VERSION,
            spans,
        }
    }
}

/// A W3C-ID-compatible, attribute-free span accepted by Studio.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DistributedTraceSpanV1 {
    /// Lowercase 32-hex-character trace identifier.
    pub trace_id: String,
    /// Lowercase 16-hex-character span identifier.
    pub span_id: String,
    /// Optional lowercase 16-hex-character parent span identifier.
    pub parent_span_id: Option<String>,
    /// Low-cardinality, caller-redacted operation name.
    pub operation: String,
    /// Bounded operation category.
    pub kind: DistributedTraceKind,
    /// Start time in Unix microseconds.
    pub started_at_unix_us: u64,
    /// Measured duration in microseconds.
    pub duration_us: u64,
    /// Terminal span status without an error body.
    pub status: DistributedTraceStatus,
}

/// Categories supported by the Studio distributed trace contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DistributedTraceKind {
    /// HTTP server or client work.
    Http,
    /// Database work identified by a redacted operation label.
    Sql,
    /// AI provider or local inference work.
    Ai,
    /// Background job work.
    Job,
    /// Cache operation.
    Cache,
    /// Explicit application operation outside the built-in categories.
    Application,
}

/// Bounded terminal status carried by a distributed span.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DistributedTraceStatus {
    /// No terminal status was recorded.
    Unset,
    /// The operation completed successfully.
    Ok,
    /// The operation failed; error details are deliberately excluded.
    Error,
}

/// A validated span retained by Studio with authenticated source metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredDistributedTraceSpan {
    /// Authenticated producer name from the signed request.
    pub source: String,
    /// Time at which Studio accepted the batch, in Unix seconds.
    pub received_at_unix_s: u64,
    /// Caller-redacted span.
    pub span: DistributedTraceSpanV1,
}

/// Kind of bounded SQL diagnostic inferred by Studio.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum QueryFindingKind {
    /// The same operation label appeared repeatedly in one trace.
    RepeatedOperation,
    /// One SQL span exceeded the fixed slow-query threshold.
    SlowOperation,
}

/// Heuristic SQL diagnostic derived without query text or bindings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryFinding {
    /// Finding category.
    pub kind: QueryFindingKind,
    /// Authenticated producer name.
    pub source: String,
    /// Trace containing the observation.
    pub trace_id: String,
    /// Caller-redacted operation label.
    pub operation: String,
    /// Number of matching observations.
    pub occurrences: usize,
    /// Highest observed duration in microseconds.
    pub maximum_duration_us: u64,
}
