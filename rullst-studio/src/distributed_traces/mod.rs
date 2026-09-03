//! Authenticated, bounded, push-only distributed trace ingestion.
//!
//! Producers submit attribute-free v1 spans through a separately mounted HMAC
//! endpoint. Studio only reads the shared in-process store through its existing
//! loopback boundary. This module does not expose remote SQL, cache values,
//! request bodies, headers, or a production administration surface.

mod auth;
mod error;
mod router;
mod store;
mod types;
mod validation;

pub use auth::{SignedTraceBatch, TraceBatchSigner, TraceIngestionKey};
pub use error::TraceIngestionError;
pub use router::{
    TRACE_NONCE_HEADER, TRACE_SIGNATURE_HEADER, TRACE_SOURCE_HEADER, TRACE_TIMESTAMP_HEADER,
    TraceIngestor,
};
pub use store::{
    DEFAULT_TRACE_STORE_CAPACITY, DistributedTraceStore, IngestionSummary,
    MAX_TRACE_STORE_CAPACITY, N_PLUS_ONE_THRESHOLD, SLOW_QUERY_THRESHOLD_US,
};
pub use types::{
    DistributedTraceKind, DistributedTraceSpanV1, DistributedTraceStatus, QueryFinding,
    QueryFindingKind, StoredDistributedTraceSpan, TRACE_BATCH_VERSION, TraceBatchV1,
};
pub use validation::{
    MAX_OPERATION_BYTES, MAX_SPAN_AGE_SECS, MAX_SPAN_DURATION_US, MAX_SPAN_FUTURE_SKEW_SECS,
    MAX_SPANS_PER_BATCH, MAX_TRACE_BATCH_BYTES,
};

#[cfg(test)]
mod tests;
