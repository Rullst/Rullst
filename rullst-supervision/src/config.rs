use crate::{OpaqueId, SupervisionError as Error};

/// Retained sessions one learner (tenant and subject) may hold by default.
pub(crate) const SUBJECT_SESSIONS: i64 = 64;

/// Hard capacities apply to the whole local store, across all tenants. One
/// learner may additionally retain at most 64 sessions (or `sessions`, when
/// lower), so a single subject cannot fill the store-wide session budget.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Limits {
    pub(crate) grants: i64,
    pub(crate) sessions: i64,
    pub(crate) events: i64,
    pub(crate) managed: i64,
    pub(crate) events_per_session: i64,
    pub(crate) event_interval: i64,
    pub(crate) subject_sessions: i64,
}

impl Limits {
    pub fn new(grants: u32, sessions: u32, events: u32, managed: u32) -> Result<Self, Error> {
        if !(1..=4096).contains(&grants)
            || !(1..=1024).contains(&sessions)
            || !(1..=16384).contains(&events)
            || !(1..=1024).contains(&managed)
        {
            return Err(Error::InvalidInput);
        }
        Ok(Self {
            grants: grants.into(),
            sessions: sessions.into(),
            events: events.into(),
            managed: managed.into(),
            events_per_session: 1024,
            event_interval: 1,
            subject_sessions: SUBJECT_SESSIONS.min(sessions.into()),
        })
    }

    pub fn event_budget(
        mut self,
        per_session: u32,
        minimum_interval_seconds: u32,
    ) -> Result<Self, Error> {
        if !(1..=2048).contains(&per_session) || !(1..=60).contains(&minimum_interval_seconds) {
            return Err(Error::InvalidInput);
        }
        self.events_per_session = per_session.into();
        self.event_interval = minimum_interval_seconds.into();
        Ok(self)
    }

    /// Sessions one learner (tenant and subject) may retain at once, from 1
    /// to the store-wide `sessions` limit (v13). The default is 64, or
    /// `sessions` when lower. Every opener must supply the same value.
    pub fn subject_sessions(mut self, maximum: u32) -> Result<Self, Error> {
        if maximum == 0 || i64::from(maximum) > self.sessions {
            return Err(Error::InvalidInput);
        }
        self.subject_sessions = maximum.into();
        Ok(self)
    }

    /// Whether the per-learner quota is the default derived from `sessions`.
    pub(crate) fn default_subject_sessions(&self) -> bool {
        self.subject_sessions == SUBJECT_SESSIONS.min(self.sessions)
    }
}

/// Immutable initialization configuration; every opener must supply the same
/// independently retained epoch and bounds. Epochs do not detect same-epoch
/// backup rollback. Retention is logical deletion, not physical secure erasure.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct StoreConfig {
    pub(crate) epoch: OpaqueId,
    pub(crate) limits: Limits,
    pub(crate) event_retention: i64,
    pub(crate) session_lifetime: i64,
}

impl StoreConfig {
    pub fn new(
        epoch: impl Into<String>,
        limits: Limits,
        event_retention_seconds: u32,
        maximum_session_seconds: u32,
    ) -> Result<Self, Error> {
        if !(3600..=604800).contains(&event_retention_seconds)
            || !(1..=28800).contains(&maximum_session_seconds)
        {
            return Err(Error::InvalidInput);
        }
        Ok(Self {
            epoch: OpaqueId::new(epoch)?,
            limits,
            event_retention: event_retention_seconds.into(),
            session_lifetime: maximum_session_seconds.into(),
        })
    }
}
