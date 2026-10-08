//! Reads the development telemetry the supervised application serves at
//! `GET /_rullst/dev-telemetry` on loopback (Rullst Core mounts it only in a
//! debug Development process started by `dev`/`dash`).
//!
//! The response is untrusted input: the body is size-bounded, the schema and
//! counters must be consistent, lists are cut to their documented sizes and
//! every displayed string loses control and bidirectional-override characters
//! before it can reach the terminal.

use serde::Deserialize;
use std::time::Duration;
use tokio::sync::mpsc;

pub(super) const TELEMETRY_PATH: &str = "/_rullst/dev-telemetry";
const SCHEMA: &str = "rullst.dev-telemetry.v1";
pub(super) const MAX_BODY_BYTES: usize = 256 * 1024;
pub(super) const MAX_REQUESTS: usize = 64;
pub(super) const MAX_SLOW_QUERIES: usize = 16;
pub(super) const MAX_REPEATED: usize = 16;
const MAX_FINGERPRINT_CHARS: usize = 96;
const MAX_PATH_CHARS: usize = 160;
const MAX_METHOD_CHARS: usize = 10;
const MAX_LABEL_CHARS: usize = 48;
/// Durations above one hour are clamped; they only scale the sparkline.
const MAX_DURATION_US: u64 = 3_600_000_000;
pub(super) const POLL_INTERVAL: Duration = Duration::from_secs(1);
const REQUEST_TIMEOUT: Duration = Duration::from_millis(800);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RequestSample {
    pub seq: u64,
    pub method: String,
    pub path: String,
    pub status: u16,
    pub duration_us: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SlowQuery {
    pub seq: u64,
    pub operation: String,
    pub model: Option<String>,
    pub table: Option<String>,
    pub duration_us: u64,
}

/// One ORM operation fingerprint the application saw repeated within a
/// single request (a possible N+1 query).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RepeatedQuery {
    pub seq: u64,
    pub method: String,
    pub route: String,
    pub fingerprint: String,
    pub occurrences: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RepeatedReport {
    Observed {
        /// Repetitions within one request at which the app reports.
        threshold: u64,
        total: u64,
        /// Oldest first, at most [`MAX_REPEATED`].
        recent: Vec<RepeatedQuery>,
    },
    /// The telemetry has no request-correlated ORM operations: an older
    /// Rullst Core, or ORM spans that are not observed.
    NotReported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HttpReport {
    pub requests_total: u64,
    pub client_errors_total: u64,
    pub server_errors_total: u64,
    /// Oldest first, at most [`MAX_REQUESTS`].
    pub recent: Vec<RequestSample>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum DatabaseReport {
    Observed {
        queries_total: u64,
        slow_total: u64,
        slow_threshold_ms: u64,
        /// Oldest first, at most [`MAX_SLOW_QUERIES`].
        recent_slow: Vec<SlowQuery>,
    },
    /// The application installed its own tracing subscriber.
    SubscriberNotInstalled,
    /// The subscriber's filter drops `rullst_orm` INFO spans.
    SpansFiltered,
    /// A state or reason this CLI does not know.
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum QueueReport {
    Observed { pending: u64 },
    NotConfigured,
    Timeout,
    DriverError,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TelemetrySnapshot {
    pub generation: String,
    pub http: HttpReport,
    pub database: DatabaseReport,
    pub repeated: RepeatedReport,
    pub queue: QueueReport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PollOutcome {
    Snapshot(TelemetrySnapshot),
    /// `404`: the application does not serve the endpoint.
    NotServed,
    /// No connection, a timeout, or a status other than success or `404`.
    Unreachable,
    /// A response that was not a valid telemetry document.
    Rejected(&'static str),
}

#[derive(Deserialize)]
struct RawPayload {
    schema: String,
    generation: String,
    http: RawHttp,
    #[serde(default)]
    database: Option<RawState>,
    #[serde(default)]
    queue: Option<RawState>,
}

#[derive(Deserialize)]
struct RawHttp {
    requests_total: u64,
    client_errors_total: u64,
    server_errors_total: u64,
    #[serde(default)]
    recent: Vec<RawRequest>,
}

#[derive(Deserialize)]
struct RawRequest {
    seq: u64,
    method: String,
    path: String,
    status: u16,
    duration_us: u64,
}

#[derive(Deserialize)]
struct RawState {
    state: String,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    pending: Option<u64>,
    #[serde(default)]
    queries_total: Option<u64>,
    #[serde(default)]
    slow_queries_total: Option<u64>,
    #[serde(default)]
    slow_threshold_ms: Option<u64>,
    #[serde(default)]
    recent_slow: Vec<RawSlow>,
    #[serde(default)]
    repeated_threshold: Option<u64>,
    #[serde(default)]
    repeated_queries_total: Option<u64>,
    #[serde(default)]
    recent_repeated: Vec<RawRepeated>,
}

#[derive(Deserialize)]
struct RawRepeated {
    seq: u64,
    method: String,
    route: String,
    fingerprint: String,
    occurrences: u64,
}

#[derive(Deserialize)]
struct RawSlow {
    seq: u64,
    operation: String,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    table: Option<String>,
    duration_us: u64,
}

/// Parses and validates one telemetry document.
pub(super) fn parse_payload(body: &[u8]) -> Result<TelemetrySnapshot, &'static str> {
    if body.len() > MAX_BODY_BYTES {
        return Err("response too large");
    }
    let mut raw: RawPayload = serde_json::from_slice(body).map_err(|_| "malformed JSON")?;
    if raw.schema != SCHEMA {
        return Err("unsupported schema");
    }
    if !is_generation(&raw.generation) {
        return Err("invalid generation");
    }
    let http = raw.http;
    let errors = http
        .client_errors_total
        .checked_add(http.server_errors_total);
    if errors.is_none_or(|errors| errors > http.requests_total) {
        return Err("inconsistent counters");
    }
    let recent = newest(http.recent, MAX_REQUESTS)
        .filter(|sample| (100..=599).contains(&sample.status))
        .filter(|sample| sample.seq > 0 && sample.seq <= http.requests_total)
        .map(|sample| RequestSample {
            seq: sample.seq,
            method: clean(&sample.method, MAX_METHOD_CHARS),
            path: clean(&sample.path, MAX_PATH_CHARS),
            status: sample.status,
            duration_us: sample.duration_us.min(MAX_DURATION_US),
        })
        .collect();
    Ok(TelemetrySnapshot {
        generation: raw.generation,
        http: HttpReport {
            requests_total: http.requests_total,
            client_errors_total: http.client_errors_total,
            server_errors_total: http.server_errors_total,
            recent,
        },
        repeated: raw
            .database
            .as_mut()
            .map_or(RepeatedReport::NotReported, repeated),
        database: raw.database.map_or(DatabaseReport::Unknown, database),
        queue: raw.queue.map_or(QueueReport::Unknown, queue),
    })
}

fn database(raw: RawState) -> DatabaseReport {
    match (raw.state.as_str(), raw.reason.as_deref()) {
        ("observed", _) => {
            let (Some(queries_total), Some(slow_total), Some(slow_threshold_ms)) = (
                raw.queries_total,
                raw.slow_queries_total,
                raw.slow_threshold_ms,
            ) else {
                return DatabaseReport::Unknown;
            };
            if slow_total > queries_total {
                return DatabaseReport::Unknown;
            }
            let recent_slow = newest(raw.recent_slow, MAX_SLOW_QUERIES)
                .filter(|slow| slow.seq > 0 && slow.seq <= slow_total)
                .map(|slow| SlowQuery {
                    seq: slow.seq,
                    operation: clean(&slow.operation, MAX_LABEL_CHARS),
                    model: slow.model.map(|value| clean(&value, MAX_LABEL_CHARS)),
                    table: slow.table.map(|value| clean(&value, MAX_LABEL_CHARS)),
                    duration_us: slow.duration_us.min(MAX_DURATION_US),
                })
                .collect();
            DatabaseReport::Observed {
                queries_total,
                slow_total,
                slow_threshold_ms,
                recent_slow,
            }
        }
        ("unavailable", Some("subscriber_not_installed")) => DatabaseReport::SubscriberNotInstalled,
        ("unavailable", Some("orm_spans_filtered")) => DatabaseReport::SpansFiltered,
        _ => DatabaseReport::Unknown,
    }
}

/// Possible N+1 findings of an observed database. A finding below the
/// reported threshold or outside the counter's range is dropped.
fn repeated(raw: &mut RawState) -> RepeatedReport {
    let (Some(threshold), Some(total)) = (raw.repeated_threshold, raw.repeated_queries_total)
    else {
        return RepeatedReport::NotReported;
    };
    if raw.state != "observed" || threshold < 2 {
        return RepeatedReport::NotReported;
    }
    let recent = newest(std::mem::take(&mut raw.recent_repeated), MAX_REPEATED)
        .filter(|finding| finding.seq > 0 && finding.seq <= total)
        .filter(|finding| finding.occurrences >= threshold)
        .map(|finding| RepeatedQuery {
            seq: finding.seq,
            method: clean(&finding.method, MAX_METHOD_CHARS),
            route: clean(&finding.route, MAX_PATH_CHARS),
            fingerprint: clean(&finding.fingerprint, MAX_FINGERPRINT_CHARS),
            occurrences: finding.occurrences,
        })
        .collect();
    RepeatedReport::Observed {
        threshold,
        total,
        recent,
    }
}

fn queue(raw: RawState) -> QueueReport {
    match (raw.state.as_str(), raw.reason.as_deref(), raw.pending) {
        ("observed", _, Some(pending)) => QueueReport::Observed { pending },
        ("not_configured", _, _) => QueueReport::NotConfigured,
        ("unavailable", Some("timeout"), _) => QueueReport::Timeout,
        ("unavailable", Some("driver_error"), _) => QueueReport::DriverError,
        _ => QueueReport::Unknown,
    }
}

/// The newest `limit` entries of an oldest-first list, oldest first.
fn newest<T>(entries: Vec<T>, limit: usize) -> impl Iterator<Item = T> {
    let skip = entries.len().saturating_sub(limit);
    entries.into_iter().skip(skip)
}

fn is_generation(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Replaces control and bidirectional-formatting characters (which could
/// move the cursor, recolor or visually reorder the terminal) and keeps at
/// most `max_chars` characters, marking a cut with `…`.
pub(super) fn clean(value: &str, max_chars: usize) -> String {
    let mut cleaned = String::new();
    for (index, character) in value.chars().enumerate() {
        if index == max_chars {
            cleaned.pop();
            cleaned.push('…');
            break;
        }
        let unsafe_for_terminal = character.is_control()
            || matches!(character, '\u{200e}' | '\u{200f}' | '\u{061c}')
            || ('\u{202a}'..='\u{202e}').contains(&character)
            || ('\u{2066}'..='\u{2069}').contains(&character);
        cleaned.push(if unsafe_for_terminal { '?' } else { character });
    }
    cleaned
}

fn client() -> Option<reqwest::Client> {
    reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(REQUEST_TIMEOUT)
        .build()
        .ok()
}

/// Polls the application's loopback endpoint once.
pub(super) async fn poll(client: &reqwest::Client, port: u16) -> PollOutcome {
    let url = format!("http://127.0.0.1:{port}{TELEMETRY_PATH}");
    let Ok(mut response) = client
        .get(url)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
    else {
        return PollOutcome::Unreachable;
    };
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return PollOutcome::NotServed;
    }
    if !response.status().is_success() {
        return PollOutcome::Unreachable;
    }
    let json = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|mime| mime.trim().eq_ignore_ascii_case("application/json"));
    if !json {
        return PollOutcome::Rejected("not a JSON response");
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_BODY_BYTES as u64)
    {
        return PollOutcome::Rejected("response too large");
    }
    let mut body = Vec::new();
    loop {
        match response.chunk().await {
            Ok(Some(chunk)) if body.len() + chunk.len() > MAX_BODY_BYTES => {
                return PollOutcome::Rejected("response too large");
            }
            Ok(Some(chunk)) => body.extend_from_slice(&chunk),
            Ok(None) => break,
            Err(_) => return PollOutcome::Unreachable,
        }
    }
    match parse_payload(&body) {
        Ok(snapshot) => PollOutcome::Snapshot(snapshot),
        Err(reason) => PollOutcome::Rejected(reason),
    }
}

/// Polls once per [`POLL_INTERVAL`] until the dashboard drops its receiver.
pub(super) fn spawn_poller(port: u16, outcomes: mpsc::Sender<PollOutcome>) {
    tokio::spawn(async move {
        let Some(client) = client() else {
            let _ = outcomes
                .send(PollOutcome::Rejected("HTTP client unavailable"))
                .await;
            return;
        };
        let mut interval = tokio::time::interval(POLL_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            if outcomes.send(poll(&client, port).await).await.is_err() {
                break;
            }
        }
    });
}
