use super::{DistributedTraceSpanV1, TRACE_BATCH_VERSION, TraceBatchV1, TraceIngestionError};
use std::collections::HashSet;

/// Maximum encoded JSON body accepted by the ingestion route.
pub const MAX_TRACE_BATCH_BYTES: usize = 128 * 1024;
/// Maximum spans accepted in one authenticated batch.
pub const MAX_SPANS_PER_BATCH: usize = 128;
/// Maximum UTF-8 bytes in an operation label.
pub const MAX_OPERATION_BYTES: usize = 160;
/// Maximum duration represented by a single span.
pub const MAX_SPAN_DURATION_US: u64 = 24 * 60 * 60 * 1_000_000;
/// Oldest accepted span relative to the signed batch timestamp.
pub const MAX_SPAN_AGE_SECS: u64 = 24 * 60 * 60;
/// Future clock allowance for a span relative to the signed batch timestamp.
pub const MAX_SPAN_FUTURE_SKEW_SECS: u64 = 300;

pub(super) fn validate_batch(
    batch: &TraceBatchV1,
    signed_at_unix_s: u64,
) -> Result<(), TraceIngestionError> {
    if batch.version != TRACE_BATCH_VERSION
        || batch.spans.is_empty()
        || batch.spans.len() > MAX_SPANS_PER_BATCH
    {
        return Err(TraceIngestionError::InvalidBatch);
    }

    let mut identities = HashSet::with_capacity(batch.spans.len());
    for span in &batch.spans {
        validate_span(span, signed_at_unix_s)?;
        if !identities.insert((&span.trace_id, &span.span_id)) {
            return Err(TraceIngestionError::InvalidBatch);
        }
    }
    Ok(())
}

fn validate_span(
    span: &DistributedTraceSpanV1,
    signed_at_unix_s: u64,
) -> Result<(), TraceIngestionError> {
    if !valid_hex_id(&span.trace_id, 32)
        || !valid_hex_id(&span.span_id, 16)
        || span
            .parent_span_id
            .as_deref()
            .is_some_and(|parent| !valid_hex_id(parent, 16))
        || span.parent_span_id.as_deref() == Some(span.span_id.as_str())
        || span.operation.is_empty()
        || span.operation.len() > MAX_OPERATION_BYTES
        || span.operation.chars().any(char::is_control)
        || span.duration_us > MAX_SPAN_DURATION_US
    {
        return Err(TraceIngestionError::InvalidBatch);
    }

    let started_at_s = span.started_at_unix_us / 1_000_000;
    let earliest = signed_at_unix_s.saturating_sub(MAX_SPAN_AGE_SECS);
    let latest = signed_at_unix_s.saturating_add(MAX_SPAN_FUTURE_SKEW_SECS);
    if started_at_s < earliest || started_at_s > latest {
        return Err(TraceIngestionError::InvalidBatch);
    }
    Ok(())
}

fn valid_hex_id(value: &str, expected_len: usize) -> bool {
    value.len() == expected_len
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        && value.bytes().any(|byte| byte != b'0')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distributed_traces::{DistributedTraceKind, DistributedTraceStatus};

    fn span(now: u64) -> DistributedTraceSpanV1 {
        DistributedTraceSpanV1 {
            trace_id: "0123456789abcdef0123456789abcdef".to_string(),
            span_id: "0123456789abcdef".to_string(),
            parent_span_id: None,
            operation: "users.fetch".to_string(),
            kind: DistributedTraceKind::Sql,
            started_at_unix_us: now * 1_000_000,
            duration_us: 10,
            status: DistributedTraceStatus::Ok,
        }
    }

    #[test]
    fn rejects_bad_ids_duplicates_time_and_operation_bounds() {
        let now = 1_800_000_000;
        let valid = span(now);
        assert!(validate_batch(&TraceBatchV1::new(vec![valid.clone()]), now).is_ok());

        let mut bad = valid.clone();
        bad.trace_id = "00000000000000000000000000000000".to_string();
        assert_eq!(
            validate_batch(&TraceBatchV1::new(vec![bad]), now),
            Err(TraceIngestionError::InvalidBatch)
        );

        assert_eq!(
            validate_batch(&TraceBatchV1::new(vec![valid.clone(), valid.clone()]), now),
            Err(TraceIngestionError::InvalidBatch)
        );

        let mut stale = valid.clone();
        stale.started_at_unix_us = now.saturating_sub(MAX_SPAN_AGE_SECS + 1) * 1_000_000;
        assert_eq!(
            validate_batch(&TraceBatchV1::new(vec![stale]), now),
            Err(TraceIngestionError::InvalidBatch)
        );

        let mut controlled = valid;
        controlled.operation = "users\nsecret".to_string();
        assert_eq!(
            validate_batch(&TraceBatchV1::new(vec![controlled]), now),
            Err(TraceIngestionError::InvalidBatch)
        );

        assert_eq!(
            validate_batch(&TraceBatchV1::new(Vec::new()), now),
            Err(TraceIngestionError::InvalidBatch)
        );
        let too_many = vec![span(now); MAX_SPANS_PER_BATCH + 1];
        assert_eq!(
            validate_batch(&TraceBatchV1::new(too_many), now),
            Err(TraceIngestionError::InvalidBatch)
        );
        let wrong_version = TraceBatchV1 {
            version: TRACE_BATCH_VERSION + 1,
            spans: vec![span(now)],
        };
        assert_eq!(
            validate_batch(&wrong_version, now),
            Err(TraceIngestionError::InvalidBatch)
        );
    }
}
