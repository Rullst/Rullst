//! Visible, participant-controlled sessions. Reports never establish misconduct.
use crate::{OpaqueId, Revision, Scope, SupervisionError as Error};

mod collection;
mod observation;
pub use collection::{Capability, Collection};
pub use observation::{
    AudioObservation, BrowserEvent, CaptureDevice, CaptureEvent, Observation, ObservationReceipt,
    ObservationRequest, ObservationSource, PresenceObservation,
};

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ExamPolicy {
    version: OpaqueId,
    notice: OpaqueId,
    lifetime: i64,
    collection: Collection,
}

impl ExamPolicy {
    pub fn new(
        version: impl Into<String>,
        notice: impl Into<String>,
        lifetime_seconds: u32,
    ) -> Result<Self, Error> {
        if !(1..=28800).contains(&lifetime_seconds) {
            return Err(Error::InvalidInput);
        }
        Ok(Self {
            version: OpaqueId::new(version)?,
            notice: OpaqueId::new(notice)?,
            lifetime: lifetime_seconds.into(),
            collection: Collection::visibility_only(),
        })
    }
    pub fn version(&self) -> &OpaqueId {
        &self.version
    }
    pub fn notice(&self) -> &OpaqueId {
        &self.notice
    }
    pub fn lifetime_seconds(&self) -> i64 {
        self.lifetime
    }
    /// The host must disclose these categories and use a distinct notice/version.
    pub fn with_collection(mut self, collection: Collection) -> Self {
        self.collection = collection;
        self
    }
    pub fn collection(&self) -> Collection {
        self.collection
    }
}

/// Explicit acknowledgement of the server-selected policy and notice. This is
/// operational collection permission, not verified identity or a legal basis.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Acknowledgement {
    policy: OpaqueId,
    notice: OpaqueId,
    collection: Collection,
}

impl Acknowledgement {
    pub fn new(
        policy: impl Into<String>,
        notice: impl Into<String>,
        acknowledged: bool,
    ) -> Result<Self, Error> {
        if !acknowledged {
            return Err(Error::InvalidInput);
        }
        Ok(Self {
            policy: OpaqueId::new(policy)?,
            notice: OpaqueId::new(notice)?,
            collection: Collection::visibility_only(),
        })
    }
    pub fn policy(&self) -> &OpaqueId {
        &self.policy
    }
    pub fn notice(&self) -> &OpaqueId {
        &self.notice
    }
    pub fn for_collection(
        policy: impl Into<String>,
        notice: impl Into<String>,
        collection: Collection,
        acknowledged: bool,
    ) -> Result<Self, Error> {
        let mut acknowledgement = Self::new(policy, notice, acknowledged)?;
        acknowledgement.collection = collection;
        Ok(acknowledgement)
    }
    pub fn collection(&self) -> Collection {
        self.collection
    }
    #[cfg(feature = "sqlite")]
    pub(crate) fn matches(
        &self,
        policy: &OpaqueId,
        notice: &OpaqueId,
        collection: Collection,
    ) -> Result<(), Error> {
        if &self.policy != policy || &self.notice != notice || self.collection != collection {
            return Err(Error::Conflict);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SessionState {
    Active,
    Paused,
    Ended,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum VisibilityEvent {
    PageVisible,
    PageHidden,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Session {
    pub(crate) id: OpaqueId,
    pub(crate) scope: Scope,
    pub(crate) policy: OpaqueId,
    pub(crate) notice: OpaqueId,
    pub(crate) collection: Collection,
    pub(crate) initial_collection: Collection,
    pub(crate) state: SessionState,
    pub(crate) revision: Revision,
    pub(crate) started_at: i64,
    pub(crate) expires_at: i64,
    pub(crate) retain_until: i64,
    pub(crate) sequence: i64,
    pub(crate) last_event_at: Option<i64>,
    pub(crate) event_count: i64,
}

impl Session {
    pub fn id(&self) -> &OpaqueId {
        &self.id
    }
    pub fn scope(&self) -> &Scope {
        &self.scope
    }
    pub fn policy(&self) -> &OpaqueId {
        &self.policy
    }
    pub fn notice(&self) -> &OpaqueId {
        &self.notice
    }
    pub fn collection(&self) -> Collection {
        self.collection
    }
    pub fn initial_collection(&self) -> Collection {
        self.initial_collection
    }
    pub fn state(&self) -> SessionState {
        self.state
    }
    pub fn revision(&self) -> Revision {
        self.revision
    }
    pub fn started_at(&self) -> i64 {
        self.started_at
    }
    pub fn expires_at(&self) -> i64 {
        self.expires_at
    }
    pub fn retain_until(&self) -> i64 {
        self.retain_until
    }
    pub fn last_sequence(&self) -> i64 {
        self.sequence
    }
    pub fn last_event_at(&self) -> Option<i64> {
        self.last_event_at
    }
    pub fn accepted_event_count(&self) -> i64 {
        self.event_count
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct EventReceipt {
    pub(crate) sequence: i64,
    pub(crate) event: VisibilityEvent,
    pub(crate) received_at: i64,
    pub(crate) expires_at: i64,
}

impl EventReceipt {
    pub fn sequence(&self) -> i64 {
        self.sequence
    }
    pub fn event(&self) -> VisibilityEvent {
        self.event
    }
    pub fn received_at(&self) -> i64 {
        self.received_at
    }
    pub fn expires_at(&self) -> i64 {
        self.expires_at
    }
}
