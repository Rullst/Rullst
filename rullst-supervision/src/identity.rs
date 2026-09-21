use crate::SupervisionError as Error;

/// Bounded opaque reference, not an access token. Debug output is minimized.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OpaqueId(String);

impl OpaqueId {
    pub fn new(value: impl Into<String>) -> Result<Self, Error> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 128
            || !value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':')
            })
        {
            return Err(Error::InvalidInput);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for OpaqueId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OpaqueId([redacted])")
    }
}

/// Monotonic store-issued revision. Supplying one does not grant authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Revision(i64);

impl Revision {
    pub fn new(value: i64) -> Result<Self, Error> {
        if value <= 0 {
            return Err(Error::InvalidInput);
        }
        Ok(Self(value))
    }
    pub fn value(self) -> i64 {
        self.0
    }
}

/// Host-resolved authenticated identity and current school membership.
/// This constructor does not verify either; never deserialize this from a client.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Context {
    tenant: OpaqueId,
    actor: OpaqueId,
}

impl Context {
    pub fn new(tenant: impl Into<String>, actor: impl Into<String>) -> Result<Self, Error> {
        Ok(Self {
            tenant: OpaqueId::new(tenant)?,
            actor: OpaqueId::new(actor)?,
        })
    }
    pub fn tenant(&self) -> &OpaqueId {
        &self.tenant
    }
    pub fn actor(&self) -> &OpaqueId {
        &self.actor
    }
    #[cfg(feature = "exam")]
    pub(crate) fn require_subject(&self, scope: &Scope) -> Result<(), Error> {
        if self.tenant != scope.tenant || self.actor != scope.subject {
            return Err(Error::Forbidden);
        }
        Ok(())
    }
}

/// A single tenant/learner/resource, additionally authorized by the host.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Scope {
    tenant: OpaqueId,
    subject: OpaqueId,
    resource: OpaqueId,
}

impl Scope {
    pub fn new(
        tenant: impl Into<String>,
        subject: impl Into<String>,
        resource: impl Into<String>,
    ) -> Result<Self, Error> {
        Ok(Self {
            tenant: OpaqueId::new(tenant)?,
            subject: OpaqueId::new(subject)?,
            resource: OpaqueId::new(resource)?,
        })
    }
    pub fn tenant(&self) -> &OpaqueId {
        &self.tenant
    }
    pub fn subject(&self) -> &OpaqueId {
        &self.subject
    }
    pub fn resource(&self) -> &OpaqueId {
        &self.resource
    }
}

/// Trusted administrative integration after independent relationship checks.
/// Possession/creation is not authentication; never expose an unchecked browser
/// endpoint that constructs this value. Evidence is an opaque audit reference.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Operator {
    context: Context,
    evidence: OpaqueId,
}

impl Operator {
    pub fn new(context: Context, evidence: impl Into<String>) -> Result<Self, Error> {
        Ok(Self {
            context,
            evidence: OpaqueId::new(evidence)?,
        })
    }
    pub fn context(&self) -> &Context {
        &self.context
    }
    pub fn evidence(&self) -> &OpaqueId {
        &self.evidence
    }
    #[cfg(feature = "sqlite")]
    pub(crate) fn require_scope(&self, scope: &Scope) -> Result<(), Error> {
        if self.context.tenant() != scope.tenant() {
            return Err(Error::Forbidden);
        }
        Ok(())
    }
}
