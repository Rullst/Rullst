//! Desktop notifications of fired alerts.
//!
//! The poller offers each alert it records; this module decides what the
//! browser shows, with an explicit clock: at most one notification per
//! project per quiet period. Alerts that arrive during the quiet period are
//! held and counted; the next notification for that project mentions them,
//! and once the period ends without one, a summary notification does. The
//! newest notifications wait in a bounded queue that open Studio pages read
//! through `/studio/notifications`; the browser shows them with its
//! Notifications API once the developer allowed it.
//!
//! Notification text is the alert's own text (Studio's wording with the
//! project name and numbers), never a label from a remote document.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, PoisonError};

/// Minutes between two notifications of one project unless changed.
pub(crate) const DEFAULT_QUIET_MINUTES: u32 = 10;
/// Longest quiet period, in minutes.
pub(crate) const MAX_QUIET_MINUTES: u32 = 1_440;
/// Notifications kept for pages to read.
pub(crate) const MAX_QUEUED: usize = 32;

/// The developer's notification settings, kept in the project list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NotificationSettings {
    /// Whether alerts become desktop notifications.
    #[serde(default)]
    pub(crate) enabled: bool,
    /// Minutes between two notifications of one project.
    #[serde(default = "default_quiet_minutes")]
    pub(crate) quiet_minutes: u32,
}

fn default_quiet_minutes() -> u32 {
    DEFAULT_QUIET_MINUTES
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            quiet_minutes: DEFAULT_QUIET_MINUTES,
        }
    }
}

impl NotificationSettings {
    /// Whether the quiet period is 1 to 1,440 minutes.
    pub(crate) fn valid(&self) -> bool {
        (1..=MAX_QUIET_MINUTES).contains(&self.quiet_minutes)
    }

    fn quiet_seconds(&self) -> u64 {
        u64::from(self.quiet_minutes) * 60
    }
}

/// One notification for the browser to show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Notification {
    /// Increasing within one Studio process.
    pub(crate) id: u64,
    pub(crate) title: String,
    pub(crate) body: String,
}

/// When a project was last notified, and the alerts held back since.
#[derive(Debug, Clone, Copy)]
struct Gate {
    last_shown: u64,
    held: u32,
}

#[derive(Debug, Default)]
struct Inner {
    last_id: u64,
    queue: VecDeque<Notification>,
    gates: HashMap<String, Gate>,
}

impl Inner {
    fn push(&mut self, project: &str, body: String) {
        self.last_id += 1;
        self.queue.push_back(Notification {
            id: self.last_id,
            title: format!("Rullst Studio · {project}"),
            body,
        });
        while self.queue.len() > MAX_QUEUED {
            self.queue.pop_front();
        }
    }
}

/// The notification queue of one Studio process.
#[derive(Debug)]
pub(crate) struct Notifier {
    /// Random per process, so a page never takes another process's cursor
    /// as its own.
    session: String,
    inner: Mutex<Inner>,
}

impl Notifier {
    pub(crate) fn new(session: String) -> Self {
        Self {
            session,
            inner: Mutex::default(),
        }
    }

    pub(crate) fn session(&self) -> &str {
        &self.session
    }

    /// Offers the alert `message` of `project`, fired at `now` (Unix
    /// seconds).
    pub(crate) fn offer(
        &self,
        settings: &NotificationSettings,
        project: &str,
        message: &str,
        now: u64,
    ) {
        if !settings.enabled {
            return;
        }
        let mut inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(gate) = inner.gates.get_mut(project)
            && now < gate.last_shown.saturating_add(settings.quiet_seconds())
        {
            gate.held = gate.held.saturating_add(1);
            return;
        }
        let gate = Gate {
            last_shown: now,
            held: 0,
        };
        let held = inner
            .gates
            .insert(project.to_string(), gate)
            .map_or(0, |previous| previous.held);
        let body = if held == 0 {
            message.to_string()
        } else {
            format!("{message} (and {held} more since the previous notification)")
        };
        inner.push(project, body);
    }

    /// Summarizes the alerts held back for each project whose quiet period
    /// ended at `now` without another notification.
    pub(crate) fn flush(&self, settings: &NotificationSettings, now: u64) {
        if !settings.enabled {
            return;
        }
        let quiet = settings.quiet_seconds();
        let mut inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let mut due: Vec<(String, u32)> = Vec::new();
        for (project, gate) in &mut inner.gates {
            if gate.held > 0 && now >= gate.last_shown.saturating_add(quiet) {
                due.push((project.clone(), gate.held));
                *gate = Gate {
                    last_shown: now,
                    held: 0,
                };
            }
        }
        due.sort();
        for (project, held) in due {
            inner.push(
                &project,
                format!("{held} more alerts since the previous notification; see the Alerts page"),
            );
        }
    }

    /// The last notification id and the notifications after `after`. A
    /// cursor of another session (an earlier Studio process) or none starts
    /// at the last id, so a newly opened page shows nothing old.
    pub(crate) fn since(
        &self,
        session: Option<&str>,
        after: Option<u64>,
    ) -> (u64, Vec<Notification>) {
        let inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let after = match (session, after) {
            (Some(session), Some(after)) if session == self.session => after,
            _ => inner.last_id,
        };
        let pending = inner
            .queue
            .iter()
            .filter(|notification| notification.id > after)
            .cloned()
            .collect();
        (inner.last_id, pending)
    }

    /// Drops what is held for a removed project.
    pub(crate) fn forget(&self, project: &str) {
        self.inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .gates
            .remove(project);
    }
}
