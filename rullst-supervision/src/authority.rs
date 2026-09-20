use crate::{OpaqueId, Revision, Scope, SupervisionError as Error};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AuthorityAction {
    ExamReview,
    ParentalManage,
}

impl AuthorityAction {
    #[cfg(feature = "sqlite")]
    pub(crate) fn code(self) -> i64 {
        match self {
            Self::ExamReview => 1,
            Self::ParentalManage => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct AuthorityKey {
    scope: Scope,
    delegate: OpaqueId,
    action: AuthorityAction,
}

impl AuthorityKey {
    pub fn new(
        scope: Scope,
        delegate: impl Into<String>,
        action: AuthorityAction,
    ) -> Result<Self, Error> {
        Ok(Self {
            scope,
            delegate: OpaqueId::new(delegate)?,
            action,
        })
    }
    pub fn scope(&self) -> &Scope {
        &self.scope
    }
    pub fn delegate(&self) -> &OpaqueId {
        &self.delegate
    }
    pub fn action(&self) -> AuthorityAction {
        self.action
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct AuthorityGrant {
    pub(crate) key: AuthorityKey,
    pub(crate) revision: Revision,
    pub(crate) expires_at: i64,
    pub(crate) revoked: bool,
}

impl AuthorityGrant {
    pub fn key(&self) -> &AuthorityKey {
        &self.key
    }
    pub fn revision(&self) -> Revision {
        self.revision
    }
    pub fn expires_at(&self) -> i64 {
        self.expires_at
    }
    pub fn is_revoked(&self) -> bool {
        self.revoked
    }
}
