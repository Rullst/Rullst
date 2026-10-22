//! Bounded in-process aggregates behind the monitoring document.
//!
//! Requests are keyed by normalized method and matched route pattern, never
//! by raw path. At most [`MAX_ROUTES`] keys are kept; later patterns share
//! the [`OTHER_ROUTE`] key. N+1 findings keep the ORM's static label
//! fingerprint and at most [`MAX_FINDINGS`] entries.

use super::window::{RouteStats, WindowCounts};
use crate::query_patterns::{N_PLUS_ONE_THRESHOLD, repeated_operations};
use crate::server::dev_telemetry::request_scope::RequestQueries;
use axum::http::Method;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Distinct method and route pattern pairs kept.
pub(super) const MAX_ROUTES: usize = 256;
/// N+1 findings kept; the least recently seen is replaced.
pub(super) const MAX_FINDINGS: usize = 64;
/// Route of requests that matched no route (404 fallback and the like).
pub(super) const UNMATCHED_ROUTE: &str = "(unmatched)";
/// Route of requests whose pattern arrived after [`MAX_ROUTES`] were kept.
pub(super) const OTHER_ROUTE: &str = "(other)";
/// Method reported for [`OTHER_ROUTE`].
pub(super) const ANY_METHOD: &str = "*";
/// Longest route pattern kept, in bytes.
const MAX_ROUTE_BYTES: usize = 256;
/// Axum's name for the tail of a nested service, shown as `{*rest}`.
const NEST_TAIL: &str = "{*__private__axum_nest_tail_param}";

/// Normalized request methods: the standard ones, then `OTHER`, so an
/// extension method cannot create keys.
const METHODS: [&str; 10] = [
    "GET", "HEAD", "POST", "PUT", "PATCH", "DELETE", "OPTIONS", "CONNECT", "TRACE", "OTHER",
];

/// Position of `method` in [`METHODS`].
pub(super) fn method_index(method: &Method) -> usize {
    match *method {
        Method::GET => 0,
        Method::HEAD => 1,
        Method::POST => 2,
        Method::PUT => 3,
        Method::PATCH => 4,
        Method::DELETE => 5,
        Method::OPTIONS => 6,
        Method::CONNECT => 7,
        Method::TRACE => 8,
        _ => 9,
    }
}

/// The label of the method at `index` in [`METHODS`].
pub(super) fn method_label(index: usize) -> &'static str {
    METHODS.get(index).copied().unwrap_or("OTHER")
}

/// The reported form of a matched route: control characters replaced,
/// Axum's nested-service tail shortened, cut to [`MAX_ROUTE_BYTES`].
pub(super) fn route_label(route: &str) -> String {
    let route = route.replace(NEST_TAIL, "{*rest}");
    let mut label = String::with_capacity(route.len().min(MAX_ROUTE_BYTES));
    for character in route.chars() {
        let character = if character.is_control() {
            '?'
        } else {
            character
        };
        if label.len() + character.len_utf8() > MAX_ROUTE_BYTES {
            break;
        }
        label.push(character);
    }
    label
}

/// Method label and route label of one key.
type RouteKey = (&'static str, Arc<str>);
type Stats = Arc<Mutex<RouteStats>>;

/// The statistics of one matched route, per method.
#[derive(Debug)]
struct RouteEntry {
    label: Arc<str>,
    methods: [Option<Stats>; METHODS.len()],
}

#[derive(Debug, Default)]
struct Routes {
    /// Keyed by the route as Axum matched it, so a lookup borrows it.
    by_route: HashMap<Box<str>, RouteEntry>,
    /// Method and route pairs in `by_route`.
    keys: usize,
    /// Shared by every pair beyond [`MAX_ROUTES`].
    other: Option<Stats>,
}

/// One route's statistics as the document reports them.
#[derive(Debug, Clone)]
pub(super) struct RouteSnapshot {
    pub(super) method: &'static str,
    pub(super) route: Arc<str>,
    pub(super) stats: RouteStats,
}

/// One fingerprint a route repeated within single requests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Finding {
    pub(super) method: &'static str,
    pub(super) route: Arc<str>,
    pub(super) fingerprint: String,
    /// Requests in which it repeated at least the threshold.
    pub(super) requests: u64,
    /// Most repetitions within one request.
    pub(super) max_occurrences: u64,
    /// Unix time of the latest such request, in seconds.
    pub(super) last_seen: u64,
    /// Order of the latest update, for eviction.
    sequence: u64,
}

#[derive(Debug, Default)]
struct Findings {
    entries: Vec<Finding>,
    total: u64,
    sequence: u64,
}

/// The process's monitoring aggregates.
#[derive(Debug)]
pub(crate) struct Collector {
    started: Instant,
    routes: RwLock<Routes>,
    queries: AtomicU64,
    findings: Mutex<Findings>,
}

impl Collector {
    pub(crate) fn new() -> Self {
        Self {
            started: Instant::now(),
            routes: RwLock::new(Routes::default()),
            queries: AtomicU64::new(0),
            findings: Mutex::new(Findings::default()),
        }
    }

    /// Time since the collector started; the window is measured from it.
    pub(super) fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }

    /// Records one response.
    pub(super) fn record(
        &self,
        method: &Method,
        route: Option<&str>,
        status: u16,
        latency: Duration,
        queries: &RequestQueries,
    ) {
        self.record_at(self.elapsed(), method, route, status, latency, queries);
    }

    pub(super) fn record_at(
        &self,
        now: Duration,
        method: &Method,
        route: Option<&str>,
        status: u16,
        latency: Duration,
        queries: &RequestQueries,
    ) {
        let (key, stats) = self.route_entry(method_index(method), route.unwrap_or(UNMATCHED_ROUTE));
        stats
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .record(now, status, latency);
        self.queries
            .fetch_add(queries.operations, Ordering::Relaxed);
        let repeated = repeated_operations(&queries.fingerprints, N_PLUS_ONE_THRESHOLD);
        if !repeated.is_empty() {
            let mut findings = self.findings.lock().unwrap_or_else(PoisonError::into_inner);
            for operation in repeated {
                findings.record(
                    &key,
                    operation.fingerprint,
                    u64::try_from(operation.occurrences).unwrap_or(u64::MAX),
                    unix_seconds(),
                );
            }
        }
    }

    /// The key and statistics of `route` (as matched) and `method`. A pair
    /// is created while fewer than [`MAX_ROUTES`] exist; later pairs share
    /// the [`OTHER_ROUTE`] statistics. The common case takes a read lock
    /// and allocates nothing.
    fn route_entry(&self, method: usize, route: &str) -> (RouteKey, Stats) {
        let found = |routes: &Routes| {
            let entry = routes.by_route.get(route)?;
            let stats = entry.methods.get(method)?.as_ref()?;
            Some((
                (method_label(method), Arc::clone(&entry.label)),
                Arc::clone(stats),
            ))
        };
        if let Some(found) = found(&self.routes.read().unwrap_or_else(PoisonError::into_inner)) {
            return found;
        }
        let mut routes = self.routes.write().unwrap_or_else(PoisonError::into_inner);
        if let Some(found) = found(&routes) {
            return found;
        }
        if routes.keys >= MAX_ROUTES {
            let other = Arc::clone(routes.other.get_or_insert_with(Stats::default));
            return ((ANY_METHOD, Arc::from(OTHER_ROUTE)), other);
        }
        routes.keys += 1;
        let entry = routes
            .by_route
            .entry(Box::from(route))
            .or_insert_with(|| RouteEntry {
                label: Arc::from(route_label(route)),
                methods: Default::default(),
            });
        let stats = Stats::default();
        if let Some(slot) = entry.methods.get_mut(method) {
            *slot = Some(Arc::clone(&stats));
        }
        let key = (method_label(method), Arc::clone(&entry.label));
        (key, stats)
    }

    /// Every route's statistics, sorted by route then method, with the
    /// [`OTHER_ROUTE`] statistics last.
    pub(super) fn routes(&self) -> Vec<RouteSnapshot> {
        let mut entries: Vec<(RouteKey, Stats)> = Vec::new();
        let other = {
            let routes = self.routes.read().unwrap_or_else(PoisonError::into_inner);
            for entry in routes.by_route.values() {
                for (method, stats) in METHODS.iter().copied().zip(&entry.methods) {
                    if let Some(stats) = stats {
                        entries.push(((method, Arc::clone(&entry.label)), Arc::clone(stats)));
                    }
                }
            }
            routes.other.clone()
        };
        entries.sort_by(|((left_method, left), _), ((right_method, right), _)| {
            left.cmp(right).then_with(|| left_method.cmp(right_method))
        });
        if let Some(other) = other {
            entries.push(((ANY_METHOD, Arc::from(OTHER_ROUTE)), other));
        }
        entries
            .into_iter()
            .map(|((method, route), stats)| RouteSnapshot {
                method,
                route,
                stats: stats.lock().unwrap_or_else(PoisonError::into_inner).clone(),
            })
            .collect()
    }

    /// Whether a route pattern was counted under [`OTHER_ROUTE`].
    pub(super) fn overflowed(&self) -> bool {
        self.routes
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .other
            .is_some()
    }

    /// Outermost ORM operations started by recorded requests.
    pub(super) fn queries(&self) -> u64 {
        self.queries.load(Ordering::Relaxed)
    }

    /// N+1 findings since start (one per fingerprint and request) and the
    /// kept entries, most recently seen first.
    pub(super) fn findings(&self) -> (u64, Vec<Finding>) {
        let findings = self.findings.lock().unwrap_or_else(PoisonError::into_inner);
        let mut entries = findings.entries.clone();
        entries.sort_by_key(|entry| std::cmp::Reverse(entry.sequence));
        (findings.total, entries)
    }
}

impl Findings {
    fn record(&mut self, key: &RouteKey, fingerprint: String, occurrences: u64, now: u64) {
        self.total = self.total.saturating_add(1);
        self.sequence = self.sequence.saturating_add(1);
        let sequence = self.sequence;
        let (method, route) = key;
        if let Some(entry) = self.entries.iter_mut().find(|entry| {
            entry.method == *method && entry.route == *route && entry.fingerprint == fingerprint
        }) {
            entry.requests = entry.requests.saturating_add(1);
            entry.max_occurrences = entry.max_occurrences.max(occurrences);
            entry.last_seen = now;
            entry.sequence = sequence;
            return;
        }
        if self.entries.len() >= MAX_FINDINGS
            && let Some(oldest) = self
                .entries
                .iter()
                .enumerate()
                .min_by_key(|(_, entry)| entry.sequence)
                .map(|(index, _)| index)
        {
            self.entries.swap_remove(oldest);
        }
        self.entries.push(Finding {
            method,
            route: Arc::clone(route),
            fingerprint,
            requests: 1,
            max_occurrences: occurrences,
            last_seen: now,
            sequence,
        });
    }
}

/// Window counts of all routes together.
pub(super) fn total_window(routes: &[RouteSnapshot], now: Duration) -> WindowCounts {
    routes
        .iter()
        .fold(WindowCounts::default(), |mut total, route| {
            total.add(&route.stats.window(now));
            total
        })
}

pub(super) fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}
