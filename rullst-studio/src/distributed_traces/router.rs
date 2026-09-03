use super::auth::{ReplayGuard, authenticate, unix_time};
use super::validation::{MAX_TRACE_BATCH_BYTES, validate_batch};
use super::{DistributedTraceStore, TraceBatchV1, TraceIngestionError, TraceIngestionKey};
use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use serde::Serialize;
use std::sync::Arc;

/// Header carrying the authenticated producer name.
pub const TRACE_SOURCE_HEADER: &str = "x-rullst-trace-source";
/// Header carrying the signed Unix-seconds timestamp.
pub const TRACE_TIMESTAMP_HEADER: &str = "x-rullst-trace-timestamp";
/// Header carrying a one-time 128-bit base64url nonce.
pub const TRACE_NONCE_HEADER: &str = "x-rullst-trace-nonce";
/// Header carrying the base64url HMAC-SHA256 signature.
pub const TRACE_SIGNATURE_HEADER: &str = "x-rullst-trace-signature";

struct IngestState {
    source: String,
    key: TraceIngestionKey,
    store: DistributedTraceStore,
    replay: ReplayGuard,
}

/// Push-only, HMAC-authenticated distributed trace ingestion endpoint.
///
/// Mount the returned router on an application path that is reachable by the
/// trusted producers. It contains no Studio read, database, cache, or mutation
/// routes; the viewer remains behind [`crate::LocalStudioAccess`].
#[derive(Clone)]
pub struct TraceIngestor {
    state: Arc<IngestState>,
}

impl std::fmt::Debug for TraceIngestor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TraceIngestor")
            .field("store", &self.state.store)
            .finish_non_exhaustive()
    }
}

impl TraceIngestor {
    /// Creates an ingestor that writes to an explicitly shared bounded store.
    pub fn new(
        store: DistributedTraceStore,
        source: impl Into<String>,
        key: TraceIngestionKey,
    ) -> Result<Self, TraceIngestionError> {
        let source = source.into();
        super::auth::validate_source(&source)?;
        Ok(Self {
            state: Arc::new(IngestState {
                source,
                key,
                store,
                replay: ReplayGuard::default(),
            }),
        })
    }

    /// Builds a router accepting `POST /` only.
    pub fn router(&self) -> Router {
        Router::new()
            .route("/", post(ingest_batch))
            .layer(DefaultBodyLimit::max(MAX_TRACE_BATCH_BYTES))
            .with_state(Arc::clone(&self.state))
    }
}

#[derive(Debug, Serialize)]
struct IngestResponse {
    accepted: usize,
    duplicates: usize,
}

async fn ingest_batch(
    State(state): State<Arc<IngestState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    match ingest(&state, &headers, &body) {
        Ok(summary) => (
            StatusCode::ACCEPTED,
            Json(IngestResponse {
                accepted: summary.accepted,
                duplicates: summary.duplicates,
            }),
        )
            .into_response(),
        Err(error) => error_response(error),
    }
}

fn ingest(
    state: &IngestState,
    headers: &HeaderMap,
    body: &[u8],
) -> Result<super::IngestionSummary, TraceIngestionError> {
    let source = required_header(headers, TRACE_SOURCE_HEADER)?;
    if source != state.source {
        return Err(TraceIngestionError::AuthenticationFailed);
    }
    let timestamp_text = required_header(headers, TRACE_TIMESTAMP_HEADER)?;
    let nonce = required_header(headers, TRACE_NONCE_HEADER)?;
    let signature = required_header(headers, TRACE_SIGNATURE_HEADER)?;
    let now = unix_time()?;
    let signed_at = authenticate(
        &state.key,
        source,
        timestamp_text,
        nonce,
        signature,
        body,
        now,
    )?;
    let batch: TraceBatchV1 =
        serde_json::from_slice(body).map_err(|_| TraceIngestionError::InvalidEncoding)?;
    validate_batch(&batch, signed_at)?;
    state.replay.consume(source, nonce, signed_at, now)?;
    state.store.insert_batch(source, now, batch.spans)
}

fn required_header<'a>(
    headers: &'a HeaderMap,
    name: &'static str,
) -> Result<&'a str, TraceIngestionError> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .ok_or(TraceIngestionError::AuthenticationFailed)
}

#[derive(Serialize)]
struct ErrorResponse {
    error: &'static str,
}

fn error_response(error: TraceIngestionError) -> Response {
    let (status, message) = match error {
        TraceIngestionError::ReplayDetected => {
            (StatusCode::CONFLICT, "trace ingestion replay rejected")
        }
        TraceIngestionError::InvalidBatch | TraceIngestionError::InvalidEncoding => {
            (StatusCode::UNPROCESSABLE_ENTITY, "invalid trace batch")
        }
        TraceIngestionError::StoreUnavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            "trace ingestion state unavailable",
        ),
        TraceIngestionError::InvalidKey | TraceIngestionError::InvalidCapacity => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "trace ingestion configuration invalid",
        ),
        TraceIngestionError::AuthenticationFailed
        | TraceIngestionError::TimestampOutsideWindow
        | TraceIngestionError::ClockUnavailable => (
            StatusCode::UNAUTHORIZED,
            "trace ingestion authentication failed",
        ),
        TraceIngestionError::RandomnessUnavailable => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "trace ingestion randomness unavailable",
        ),
    };
    (status, Json(ErrorResponse { error: message })).into_response()
}
