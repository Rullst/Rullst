//! Additional application restrictions; these never grant learning access.
use crate::{OpaqueId, Revision, Scope, SupervisionError as Error, clock::MAX_TIME};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CoursePolicy {
    pub(crate) courses: BTreeSet<OpaqueId>,
    pub(crate) not_before: i64,
    pub(crate) expires_at: i64,
}

impl CoursePolicy {
    /// One absolute UTC window of at most 30 days; an empty list denies all.
    pub fn new<I, S>(courses: I, not_before: i64, expires_at: i64) -> Result<Self, Error>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        if not_before < 0
            || expires_at <= not_before
            || expires_at > MAX_TIME
            || expires_at - not_before > 2_592_000
        {
            return Err(Error::InvalidInput);
        }
        let mut set = BTreeSet::new();
        for value in courses {
            if set.len() >= 64 || !set.insert(OpaqueId::new(value)?) {
                return Err(Error::InvalidInput);
            }
        }
        Ok(Self {
            courses: set,
            not_before,
            expires_at,
        })
    }
    pub fn courses(&self) -> &BTreeSet<OpaqueId> {
        &self.courses
    }
    pub fn not_before(&self) -> i64 {
        self.not_before
    }
    pub fn expires_at(&self) -> i64 {
        self.expires_at
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ManagedLearner {
    pub(crate) scope: Scope,
    pub(crate) revision: Revision,
    pub(crate) operator: OpaqueId,
    pub(crate) policy_actor: Option<OpaqueId>,
    pub(crate) policy: Option<CoursePolicy>,
}

impl ManagedLearner {
    pub fn scope(&self) -> &Scope {
        &self.scope
    }
    pub fn revision(&self) -> Revision {
        self.revision
    }
    pub fn operator(&self) -> &OpaqueId {
        &self.operator
    }
    pub fn policy_actor(&self) -> Option<&OpaqueId> {
        self.policy_actor.as_ref()
    }
    pub fn policy(&self) -> Option<&CoursePolicy> {
        self.policy.as_ref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AccessDecision {
    Unmanaged,
    Allowed,
    MissingPolicy,
    OutsideWindow,
    CourseDenied,
}

impl AccessDecision {
    /// Still requires the host's independent course/enrollment authorization.
    pub fn permits_learning(self) -> bool {
        matches!(self, Self::Unmanaged | Self::Allowed)
    }
}
