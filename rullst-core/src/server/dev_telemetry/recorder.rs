//! Bounded in-process aggregates behind the development telemetry endpoint.
//!
//! Only request metadata the access log already prints is kept: method, path
//! (never the query string), status and duration. ORM operations keep their
//! static span labels and duration, never SQL text or bindings.

use serde::Serialize;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant};

/// Newest requests returned per snapshot.
pub(super) const RECENT_REQUESTS: usize = 64;
/// Newest slow ORM operations returned per snapshot.
pub(super) const RECENT_SLOW_QUERIES: usize = 16;
/// Newest possible N+1 findings returned per snapshot.
pub(super) const RECENT_REPEATED: usize = 16;
/// ORM operations taking at least this long are reported as slow.
pub(super) const SLOW_QUERY_THRESHOLD: Duration = Duration::from_millis(100);
const MAX_PATH_BYTES: usize = 256;
const MAX_METHOD_BYTES: usize = 16;
const MAX_LABEL_BYTES: usize = 64;
const MAX_FINGERPRINT_BYTES: usize = 128;

static GLOBAL: OnceLock<Arc<Recorder>> = OnceLock::new();

/// The process recorder, once the development endpoint has been mounted.
pub(super) fn global() -> Option<&'static Arc<Recorder>> {
    GLOBAL.get()
}

/// Enables recording for this process and returns the shared recorder.
pub(super) fn enable_global() -> Arc<Recorder> {
    GLOBAL.get_or_init(|| Arc::new(Recorder::new())).clone()
}

/// Records one completed request when development telemetry is enabled; a
/// single atomic load otherwise.
pub(super) fn record_request(method: &str, path: &str, status: u16, elapsed: Duration) {
    if let Some(recorder) = global() {
        recorder.record_request(method, path, status, elapsed);
    }
}

/// Records the operations of one completed request that repeated at least
/// [`crate::query_patterns::N_PLUS_ONE_THRESHOLD`] times.
pub(super) fn record_repeated(method: &str, route: &str, operations: &[String]) {
    let Some(recorder) = global() else {
        return;
    };
    let repeated = crate::query_patterns::repeated_operations(
        operations,
        crate::query_patterns::N_PLUS_ONE_THRESHOLD,
    );
    if !repeated.is_empty() {
        recorder.record_repeated(method, route, repeated);
    }
}

/// One completed request as reported to the dashboard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct RequestSample {
    pub(super) seq: u64,
    pub(super) method: String,
    pub(super) path: String,
    pub(super) status: u16,
    pub(super) duration_us: u64,
}

/// Static labels of an ORM operation span.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct QueryLabels {
    pub(super) operation: Option<String>,
    pub(super) model: Option<String>,
    pub(super) table: Option<String>,
}

/// One ORM operation at or above [`SLOW_QUERY_THRESHOLD`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct SlowQuery {
    pub(super) seq: u64,
    pub(super) operation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) table: Option<String>,
    pub(super) duration_us: u64,
}

/// One ORM operation fingerprint repeated within a single request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct RepeatedQuery {
    pub(super) seq: u64,
    pub(super) method: String,
    /// The matched route template, or the path without its query string.
    pub(super) route: String,
    pub(super) fingerprint: String,
    pub(super) occurrences: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct HttpSnapshot {
    pub(super) requests_total: u64,
    pub(super) client_errors_total: u64,
    pub(super) server_errors_total: u64,
    /// Oldest first; `seq` equals `requests_total` for the newest entry.
    pub(super) recent: Vec<RequestSample>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct QuerySnapshot {
    pub(super) queries_total: u64,
    pub(super) slow_queries_total: u64,
    pub(super) slow_threshold_ms: u64,
    /// Oldest first; `seq` equals `slow_queries_total` for the newest entry.
    pub(super) recent_slow: Vec<SlowQuery>,
    /// Repetitions within one request at which a finding is recorded.
    pub(super) repeated_threshold: u64,
    /// Possible N+1 findings since start (one per fingerprint and request).
    pub(super) repeated_queries_total: u64,
    /// Oldest first; `seq` equals `repeated_queries_total` for the newest.
    pub(super) recent_repeated: Vec<RepeatedQuery>,
}

#[derive(Debug, Default)]
struct State {
    requests_total: u64,
    client_errors_total: u64,
    server_errors_total: u64,
    recent: VecDeque<RequestSample>,
    queries_total: u64,
    slow_queries_total: u64,
    slow: VecDeque<SlowQuery>,
    repeated_total: u64,
    repeated: VecDeque<RepeatedQuery>,
}

/// Process-local counters and bounded recent lists.
#[derive(Debug)]
pub(crate) struct Recorder {
    started: Instant,
    state: Mutex<State>,
}

impl Recorder {
    pub(super) fn new() -> Self {
        Self {
            started: Instant::now(),
            state: Mutex::new(State::default()),
        }
    }

    pub(super) fn uptime(&self) -> Duration {
        self.started.elapsed()
    }

    pub(super) fn record_request(&self, method: &str, path: &str, status: u16, elapsed: Duration) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.requests_total = state.requests_total.saturating_add(1);
        match status {
            400..=499 => state.client_errors_total = state.client_errors_total.saturating_add(1),
            500..=599 => state.server_errors_total = state.server_errors_total.saturating_add(1),
            _ => {}
        }
        let sample = RequestSample {
            seq: state.requests_total,
            method: bounded_text(method, MAX_METHOD_BYTES),
            path: bounded_text(path, MAX_PATH_BYTES),
            status,
            duration_us: micros(elapsed),
        };
        push_bounded(&mut state.recent, sample, RECENT_REQUESTS);
    }

    pub(super) fn record_query(&self, labels: QueryLabels, elapsed: Duration) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.queries_total = state.queries_total.saturating_add(1);
        if elapsed < SLOW_QUERY_THRESHOLD {
            return;
        }
        state.slow_queries_total = state.slow_queries_total.saturating_add(1);
        let label =
            |value: Option<String>| value.map(|value| bounded_text(&value, MAX_LABEL_BYTES));
        let slow = SlowQuery {
            seq: state.slow_queries_total,
            operation: label(labels.operation).unwrap_or_else(|| "unknown".to_string()),
            model: label(labels.model),
            table: label(labels.table),
            duration_us: micros(elapsed),
        };
        push_bounded(&mut state.slow, slow, RECENT_SLOW_QUERIES);
    }

    pub(super) fn record_repeated(
        &self,
        method: &str,
        route: &str,
        repeated: Vec<crate::query_patterns::RepeatedOperation>,
    ) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        for operation in repeated {
            state.repeated_total = state.repeated_total.saturating_add(1);
            let finding = RepeatedQuery {
                seq: state.repeated_total,
                method: bounded_text(method, MAX_METHOD_BYTES),
                route: bounded_text(route, MAX_PATH_BYTES),
                fingerprint: bounded_text(&operation.fingerprint, MAX_FINGERPRINT_BYTES),
                occurrences: u64::try_from(operation.occurrences).unwrap_or(u64::MAX),
            };
            push_bounded(&mut state.repeated, finding, RECENT_REPEATED);
        }
    }

    pub(super) fn http_snapshot(&self) -> HttpSnapshot {
        let state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        HttpSnapshot {
            requests_total: state.requests_total,
            client_errors_total: state.client_errors_total,
            server_errors_total: state.server_errors_total,
            recent: state.recent.iter().cloned().collect(),
        }
    }

    pub(super) fn query_snapshot(&self) -> QuerySnapshot {
        let state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        QuerySnapshot {
            queries_total: state.queries_total,
            slow_queries_total: state.slow_queries_total,
            slow_threshold_ms: u64::try_from(SLOW_QUERY_THRESHOLD.as_millis()).unwrap_or(u64::MAX),
            recent_slow: state.slow.iter().cloned().collect(),
            repeated_threshold: u64::try_from(crate::query_patterns::N_PLUS_ONE_THRESHOLD)
                .unwrap_or(u64::MAX),
            repeated_queries_total: state.repeated_total,
            recent_repeated: state.repeated.iter().cloned().collect(),
        }
    }
}

fn push_bounded<T>(entries: &mut VecDeque<T>, entry: T, capacity: usize) {
    while entries.len() >= capacity {
        entries.pop_front();
    }
    entries.push_back(entry);
}

pub(super) fn micros(elapsed: Duration) -> u64 {
    u64::try_from(elapsed.as_micros()).unwrap_or(u64::MAX)
}

/// Keeps at most `max_bytes` of whole characters and replaces control
/// characters, so a recorded value cannot carry terminal escape sequences.
pub(super) fn bounded_text(value: &str, max_bytes: usize) -> String {
    let mut text = String::with_capacity(value.len().min(max_bytes));
    for character in value.chars() {
        let character = if character.is_control() {
            '?'
        } else {
            character
        };
        if text.len() + character.len_utf8() > max_bytes {
            break;
        }
        text.push(character);
    }
    text
}
