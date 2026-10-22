//! What the remote Studio remembers about each project between page views.

use super::client::FetchError;
use super::document::MonitorDocument;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// The outcome of the latest read of one project.
#[derive(Debug, Clone, PartialEq, Eq)]
struct LastRead {
    /// Unix time of the read, in seconds.
    at: u64,
    /// `None` on success, otherwise the fixed error status.
    error: Option<String>,
}

/// Process-local monitoring state shared by the pages.
#[derive(Debug, Default)]
pub(crate) struct Monitoring {
    last: Mutex<HashMap<String, LastRead>>,
    /// Unacknowledged alerts, refreshed after each poll and acknowledgement.
    open_alerts: AtomicU64,
}

pub(crate) fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

impl Monitoring {
    /// Records the outcome of a read of `project`.
    pub(crate) fn observe(&self, project: &str, outcome: &Result<MonitorDocument, FetchError>) {
        let read = LastRead {
            at: unix_now(),
            error: outcome.as_ref().err().map(FetchError::message),
        };
        self.last
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .insert(project.to_string(), read);
    }

    /// A one-line status of the latest read of `project`.
    pub(crate) fn last_status(&self, project: &str) -> Option<String> {
        let last = self.last.lock().unwrap_or_else(|error| error.into_inner());
        last.get(project).map(|read| match &read.error {
            None => format!("Last read succeeded at Unix time {}.", read.at),
            Some(error) => format!("Last read failed at Unix time {}: {error}.", read.at),
        })
    }

    /// Alerts not acknowledged yet, as last counted.
    pub(crate) fn open_alerts(&self) -> u64 {
        self.open_alerts.load(Ordering::Relaxed)
    }

    pub(crate) fn set_open_alerts(&self, count: u64) {
        self.open_alerts.store(count, Ordering::Relaxed);
    }

    /// Drops what is remembered about a removed project.
    pub(crate) fn forget(&self, project: &str) {
        self.last
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .remove(project);
    }
}
