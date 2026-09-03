use super::{
    DistributedTraceKind, QueryFinding, QueryFindingKind, StoredDistributedTraceSpan,
    TraceIngestionError,
};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, RwLock};

/// Maximum number of spans retained by one in-process store.
pub const MAX_TRACE_STORE_CAPACITY: usize = 10_000;
/// Default number of distributed spans retained by Studio.
pub const DEFAULT_TRACE_STORE_CAPACITY: usize = 2_048;
/// SQL duration at which the local heuristic reports a slow operation.
pub const SLOW_QUERY_THRESHOLD_US: u64 = 100_000;
/// Repetition count at which the local heuristic reports a possible N+1 pattern.
pub const N_PLUS_ONE_THRESHOLD: usize = 3;

#[derive(Default)]
struct StoreInner {
    spans: VecDeque<StoredDistributedTraceSpan>,
    identities: HashSet<(String, String)>,
}

/// Bounded in-process store shared by an authenticated ingestion endpoint and
/// a loopback-only Studio viewer.
#[derive(Clone)]
pub struct DistributedTraceStore {
    inner: Arc<RwLock<StoreInner>>,
    capacity: usize,
}

impl std::fmt::Debug for DistributedTraceStore {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DistributedTraceStore")
            .field("capacity", &self.capacity)
            .finish_non_exhaustive()
    }
}

impl Default for DistributedTraceStore {
    fn default() -> Self {
        Self {
            inner: Arc::new(RwLock::new(StoreInner::default())),
            capacity: DEFAULT_TRACE_STORE_CAPACITY,
        }
    }
}

impl DistributedTraceStore {
    /// Creates a store with a fixed non-zero capacity no larger than 10,000.
    pub fn new(capacity: usize) -> Result<Self, TraceIngestionError> {
        if capacity == 0 || capacity > MAX_TRACE_STORE_CAPACITY {
            return Err(TraceIngestionError::InvalidCapacity);
        }
        Ok(Self {
            inner: Arc::new(RwLock::new(StoreInner::default())),
            capacity,
        })
    }

    pub(crate) fn insert_batch(
        &self,
        source: &str,
        received_at_unix_s: u64,
        spans: Vec<super::DistributedTraceSpanV1>,
    ) -> Result<IngestionSummary, TraceIngestionError> {
        let mut state = self
            .inner
            .write()
            .map_err(|_| TraceIngestionError::StoreUnavailable)?;
        let mut accepted = 0_usize;
        let mut duplicates = 0_usize;

        for span in spans {
            let identity = (span.trace_id.clone(), span.span_id.clone());
            if state.identities.contains(&identity) {
                duplicates += 1;
                continue;
            }
            while state.spans.len() >= self.capacity {
                if let Some(evicted) = state.spans.pop_front() {
                    state
                        .identities
                        .remove(&(evicted.span.trace_id, evicted.span.span_id));
                }
            }
            state.identities.insert(identity);
            state.spans.push_back(StoredDistributedTraceSpan {
                source: source.to_string(),
                received_at_unix_s,
                span,
            });
            accepted += 1;
        }

        Ok(IngestionSummary {
            accepted,
            duplicates,
        })
    }

    /// Returns a stable copy of every retained distributed span.
    pub fn snapshot(&self) -> Result<Vec<StoredDistributedTraceSpan>, TraceIngestionError> {
        self.inner
            .read()
            .map(|state| state.spans.iter().cloned().collect())
            .map_err(|_| TraceIngestionError::StoreUnavailable)
    }

    /// Derives bounded slow-query and repeated-operation diagnostics.
    ///
    /// Repetition is only a heuristic: three equal operation labels in one
    /// trace can be intentional and do not prove an N+1 defect.
    pub fn query_findings(&self) -> Result<Vec<QueryFinding>, TraceIngestionError> {
        let spans = self.snapshot()?;
        let mut groups: HashMap<(String, String, String), (usize, u64)> = HashMap::new();
        let mut slow = Vec::new();

        for stored in spans {
            if stored.span.kind != DistributedTraceKind::Sql {
                continue;
            }
            if stored.span.duration_us >= SLOW_QUERY_THRESHOLD_US {
                slow.push(QueryFinding {
                    kind: QueryFindingKind::SlowOperation,
                    source: stored.source.clone(),
                    trace_id: stored.span.trace_id.clone(),
                    operation: stored.span.operation.clone(),
                    occurrences: 1,
                    maximum_duration_us: stored.span.duration_us,
                });
            }
            let entry = groups
                .entry((stored.source, stored.span.trace_id, stored.span.operation))
                .or_default();
            entry.0 += 1;
            entry.1 = entry.1.max(stored.span.duration_us);
        }

        let mut findings = Vec::new();
        for ((source, trace_id, operation), (occurrences, maximum_duration_us)) in groups {
            if occurrences >= N_PLUS_ONE_THRESHOLD {
                findings.push(QueryFinding {
                    kind: QueryFindingKind::RepeatedOperation,
                    source,
                    trace_id,
                    operation,
                    occurrences,
                    maximum_duration_us,
                });
            }
        }
        findings.extend(slow);
        findings.sort_by(|left, right| {
            left.trace_id
                .cmp(&right.trace_id)
                .then_with(|| left.operation.cmp(&right.operation))
                .then_with(|| left.kind.cmp(&right.kind))
        });
        Ok(findings)
    }
}

/// Counts returned after a completely validated batch is committed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IngestionSummary {
    /// Newly retained spans.
    pub accepted: usize,
    /// Already-retained trace/span identities skipped idempotently.
    pub duplicates: usize,
}
