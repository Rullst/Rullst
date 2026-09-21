use crate::security::TenantContext;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt, future::Future};

/// Secret-minimized failures at the recoverable Live boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LiveRecoveryError {
    /// The authenticated session or current scope no longer permits this view.
    #[error("live access denied")]
    Unauthorized,
    /// The domain revision changed before the transaction could apply an action.
    #[error("live revision conflict")]
    Conflict,
    /// Configuration, a command or a snapshot violates the bounded protocol.
    #[error("invalid live value")]
    Invalid,
    /// A required application dependency or operation is unavailable.
    #[error("live application unavailable")]
    Unavailable,
}

/// Result of a bounded Live application operation.
pub type LiveResult<T> = Result<T, LiveRecoveryError>;

/// Immutable application-authenticated tenant, account and component binding.
///
/// Construct from server authentication and membership, never browser claims.
/// This value is a binding, not a substitute for the application's `authorize`.
#[derive(Clone, PartialEq, Eq)]
pub struct LiveScope {
    tenant: String,
    account: String,
    component: String,
}

impl LiveScope {
    /// Binds an already-authorized tenant context to one account and view key.
    pub fn try_new(
        context: &TenantContext,
        account: impl Into<String>,
        component: impl Into<String>,
    ) -> LiveResult<Self> {
        let account = account.into();
        let component = component.into();
        identifier(&account, 128)?;
        identifier(&component, 128)?;
        let tenant = TenantContext::try_new(context.tenant_id.clone())
            .map_err(|_| LiveRecoveryError::Invalid)?;
        Ok(Self {
            tenant: tenant.tenant_id,
            account,
            component,
        })
    }

    /// Returns the trusted tenant binding.
    pub fn tenant(&self) -> &str {
        &self.tenant
    }
    /// Returns the trusted account binding.
    pub fn account(&self) -> &str {
        &self.account
    }
    /// Returns the application-owned component/resource binding.
    pub fn component(&self) -> &str {
        &self.component
    }
}

impl fmt::Debug for LiveScope {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("LiveScope([REDACTED])")
    }
}

/// A complete authoritative view, with at most 64 KiB of trusted rendered HTML.
/// Escape untrusted domain values before constructing the snapshot.
#[derive(Clone)]
pub struct LiveSnapshot {
    revision: u64,
    html: String,
}

impl LiveSnapshot {
    /// Creates a full snapshot. Revisions are serialized as decimal strings.
    pub fn try_new(revision: u64, html: impl Into<String>) -> LiveResult<Self> {
        let html = html.into();
        if html.len() > 65536 {
            return Err(LiveRecoveryError::Invalid);
        }
        Ok(Self { revision, html })
    }
    /// Returns the authoritative domain revision.
    pub fn revision(&self) -> u64 {
        self.revision
    }
    /// Returns the server-rendered HTML contents for the mounted view.
    pub fn html(&self) -> &str {
        &self.html
    }
}

impl fmt::Debug for LiveSnapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LiveSnapshot")
            .field("revision", &self.revision)
            .field("html_bytes", &self.html.len())
            .finish()
    }
}

/// Validated command. Its fields are untrusted input, never scope/role assertions.
#[derive(Clone)]
pub struct LiveCommand {
    id: String,
    revision: u64,
    action: String,
    fields: BTreeMap<String, String>,
}

impl LiveCommand {
    /// Returns the bounded browser correlation ID, not an authentication token.
    pub fn id(&self) -> &str {
        &self.id
    }
    /// Returns the revision that the domain transaction must compare atomically.
    pub fn expected_revision(&self) -> u64 {
        self.revision
    }
    /// Returns the requested application action.
    pub fn action(&self) -> &str {
        &self.action
    }
    /// Returns the untrusted, bounded named form fields.
    pub fn fields(&self) -> &BTreeMap<String, String> {
        &self.fields
    }

    pub(super) fn decode(text: &str) -> LiveResult<Self> {
        if text.len() > 16384 {
            return Err(LiveRecoveryError::Invalid);
        }
        let wire: CommandWire =
            serde_json::from_str(text).map_err(|_| LiveRecoveryError::Invalid)?;
        if wire.version != 1
            || wire.kind != "action"
            || wire.id.len() != 32
            || !wire
                .id
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            || wire.revision.len() > 20
            || (wire.revision.len() > 1 && wire.revision.starts_with('0'))
            || wire.revision.is_empty()
            || !wire.revision.bytes().all(|byte| byte.is_ascii_digit())
            || wire.fields.len() > 32
        {
            return Err(LiveRecoveryError::Invalid);
        }
        identifier(&wire.action, 64)?;
        for (name, value) in &wire.fields {
            identifier(name, 64)?;
            if value.len() > 2048 || value.contains('\0') {
                return Err(LiveRecoveryError::Invalid);
            }
        }
        Ok(Self {
            id: wire.id,
            revision: wire
                .revision
                .parse()
                .map_err(|_| LiveRecoveryError::Invalid)?,
            action: wire.action,
            fields: wire.fields,
        })
    }
}

impl fmt::Debug for LiveCommand {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LiveCommand")
            .field("expected_revision", &self.revision)
            .field("field_count", &self.fields.len())
            .finish_non_exhaustive()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CommandWire {
    version: u8,
    kind: String,
    id: String,
    revision: String,
    action: String,
    #[serde(deserialize_with = "fields_without_duplicates")]
    fields: BTreeMap<String, String>,
}

fn fields_without_duplicates<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, String>, D::Error> {
    struct Fields;
    impl<'de> serde::de::Visitor<'de> for Fields {
        type Value = BTreeMap<String, String>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("bounded unique form fields")
        }
        fn visit_map<M: serde::de::MapAccess<'de>>(
            self,
            mut map: M,
        ) -> Result<Self::Value, M::Error> {
            use serde::de::Error;
            let mut fields = BTreeMap::new();
            while let Some((name, value)) = map.next_entry::<String, String>()? {
                if fields.len() >= 32 || fields.insert(name, value).is_some() {
                    return Err(M::Error::custom("duplicate or excessive form fields"));
                }
            }
            Ok(fields)
        }
    }
    deserializer.deserialize_map(Fields)
}

/// Static-dispatch application boundary for authoritative state and access.
pub trait RecoverableLiveView: Send + Sync + 'static {
    /// Rechecks the current session, tenant membership and view-level permission.
    fn authorize(&self, scope: &LiveScope) -> impl Future<Output = LiveResult<()>> + Send;
    /// Loads and renders complete authoritative state for this exact scope.
    fn snapshot(&self, scope: &LiveScope) -> impl Future<Output = LiveResult<LiveSnapshot>> + Send;
    /// Atomically checks `expected_revision` with the persistent domain mutation.
    /// Return `Conflict` without mutation on mismatch; accepted actions must
    /// advance the revision. Cancellation/timeout cannot prove rollback, so the
    /// browser recovers a snapshot rather than replaying an uncertain action.
    fn apply(
        &self,
        scope: &LiveScope,
        command: &LiveCommand,
    ) -> impl Future<Output = LiveResult<LiveSnapshot>> + Send;
}

#[derive(Serialize)]
pub(super) struct SnapshotWire<'a> {
    pub(super) version: u8,
    pub(super) kind: &'static str,
    pub(super) revision: String,
    pub(super) html: &'a str,
    pub(super) id: Option<&'a str>,
    pub(super) outcome: &'static str,
}

pub(super) fn identifier(value: &str, limit: usize) -> LiveResult<()> {
    if value.is_empty()
        || value.len() > limit
        || !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':' | b'/')
        })
    {
        return Err(LiveRecoveryError::Invalid);
    }
    Ok(())
}
