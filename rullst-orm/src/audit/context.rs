use crate::{RullstValue, tenant::get_tenant_id};
use serde::{Deserialize, Serialize};
use std::{fmt, future::Future};

const MAX_ACTOR_ID_BYTES: usize = 255;
const MAX_CORRELATION_ID_BYTES: usize = 255;
const MAX_REASON_BYTES: usize = 1_024;
const MAX_TENANT_KEY_BYTES: usize = 512;

tokio::task_local! {
    static CURRENT_AUDIT_CONTEXT: AuditContext;
}

/// Category of principal responsible for an audited mutation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum AuditActorKind {
    /// An authenticated human user.
    User,
    /// A named application service or worker.
    Service,
    /// An explicitly identified system process.
    System,
}

impl AuditActorKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Service => "service",
            Self::System => "system",
        }
    }
}

/// Validated identity and correlation metadata for audited mutations.
#[derive(Clone, PartialEq, Eq)]
pub struct AuditContext {
    actor_kind: AuditActorKind,
    actor_id: String,
    correlation_id: Option<String>,
    reverted_audit_id: Option<i32>,
    reason: Option<String>,
}

impl AuditContext {
    /// Creates a context for an authenticated human principal.
    pub fn user(actor_id: impl Into<String>) -> Result<Self, crate::Error> {
        Self::new(AuditActorKind::User, actor_id)
    }

    /// Creates a context for a named service or background worker.
    pub fn service(actor_id: impl Into<String>) -> Result<Self, crate::Error> {
        Self::new(AuditActorKind::Service, actor_id)
    }

    /// Creates a context for an explicitly identified system process.
    pub fn system(actor_id: impl Into<String>) -> Result<Self, crate::Error> {
        Self::new(AuditActorKind::System, actor_id)
    }

    fn new(actor_kind: AuditActorKind, actor_id: impl Into<String>) -> Result<Self, crate::Error> {
        let actor_id = actor_id.into();
        validate_text("audit actor ID", &actor_id, MAX_ACTOR_ID_BYTES)?;
        Ok(Self {
            actor_kind,
            actor_id,
            correlation_id: None,
            reverted_audit_id: None,
            reason: None,
        })
    }

    /// Adds a request, job, or trace correlation identifier.
    pub fn with_correlation_id(
        mut self,
        correlation_id: impl Into<String>,
    ) -> Result<Self, crate::Error> {
        let correlation_id = correlation_id.into();
        validate_text(
            "audit correlation ID",
            &correlation_id,
            MAX_CORRELATION_ID_BYTES,
        )?;
        self.correlation_id = Some(correlation_id);
        Ok(self)
    }

    /// Returns the principal category.
    pub fn actor_kind(&self) -> AuditActorKind {
        self.actor_kind
    }

    /// Returns the application-owned stable principal identifier.
    pub fn actor_id(&self) -> &str {
        &self.actor_id
    }

    /// Returns the optional request, job, or trace correlation identifier.
    pub fn correlation_id(&self) -> Option<&str> {
        self.correlation_id.as_deref()
    }

    /// Associates a later audit entry with an explicitly restored revision.
    #[doc(hidden)]
    pub fn for_revision_restore(
        mut self,
        audit_id: i32,
        reason: impl Into<String>,
    ) -> Result<Self, crate::Error> {
        if audit_id <= 0 {
            return Err(crate::Error::Validation(
                "restored audit ID must be positive".to_string(),
            ));
        }
        let reason = reason.into();
        validate_text("revision restore reason", &reason, MAX_REASON_BYTES)?;
        self.reverted_audit_id = Some(audit_id);
        self.reason = Some(reason);
        Ok(self)
    }
}

impl fmt::Debug for AuditContext {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuditContext")
            .field("actor_kind", &self.actor_kind)
            .field("actor_id_bytes", &self.actor_id.len())
            .field("has_correlation_id", &self.correlation_id.is_some())
            .field("reverted_audit_id", &self.reverted_audit_id)
            .field("has_reason", &self.reason.is_some())
            .finish()
    }
}

/// Runs one future with mandatory audit-principal metadata.
pub async fn with_audit_context<F, R>(context: AuditContext, future: F) -> R
where
    F: Future<Output = R>,
{
    CURRENT_AUDIT_CONTEXT.scope(context, future).await
}

/// Returns the active audit context, if the current task installed one.
pub fn current_audit_context() -> Option<AuditContext> {
    CURRENT_AUDIT_CONTEXT.try_with(Clone::clone).ok()
}

#[derive(Clone)]
pub(crate) struct AuditMetadata {
    pub actor_kind: &'static str,
    pub actor_id: String,
    pub tenant_key: Option<String>,
    pub correlation_id: Option<String>,
    pub reverted_audit_id: Option<i32>,
    pub reason: Option<String>,
}

pub(crate) fn current_metadata() -> Result<AuditMetadata, crate::Error> {
    let context = current_audit_context().ok_or_else(|| {
        crate::Error::Validation(
            "auditable mutations require with_audit_context(AuditContext, future)".to_string(),
        )
    })?;
    Ok(AuditMetadata {
        actor_kind: context.actor_kind.as_str(),
        actor_id: context.actor_id,
        tenant_key: active_tenant_key()?,
        correlation_id: context.correlation_id,
        reverted_audit_id: context.reverted_audit_id,
        reason: context.reason,
    })
}

fn active_tenant_key() -> Result<Option<String>, crate::Error> {
    let Some(tenant) = get_tenant_id() else {
        return Ok(None);
    };
    let key = match tenant {
        RullstValue::String(value) => {
            validate_text("audit tenant ID", &value, MAX_TENANT_KEY_BYTES - 7)?;
            format!("string:{value}")
        }
        RullstValue::Int(value) => format!("i32:{value}"),
        RullstValue::Float(value) if value.is_finite() => {
            format!("f64:{:016x}", value.to_bits())
        }
        RullstValue::Float(_) => {
            return Err(crate::Error::Validation(
                "audit tenant ID must be finite".to_string(),
            ));
        }
        RullstValue::Bool(value) => format!("bool:{value}"),
        RullstValue::I64(value) => format!("i64:{value}"),
        RullstValue::Null => {
            return Err(crate::Error::Validation(
                "audit tenant ID must not be NULL".to_string(),
            ));
        }
        RullstValue::DateTime(_)
        | RullstValue::NaiveDateTime(_)
        | RullstValue::Date(_)
        | RullstValue::Time(_) => {
            return Err(crate::Error::Validation(
                "audit tenant ID must not be a date or time".to_string(),
            ));
        }
    };
    if key.len() > MAX_TENANT_KEY_BYTES {
        return Err(crate::Error::Validation(
            "audit tenant key is too long".to_string(),
        ));
    }
    Ok(Some(key))
}

fn validate_text(field: &'static str, value: &str, max_bytes: usize) -> Result<(), crate::Error> {
    if value.is_empty()
        || value.len() > max_bytes
        || value.trim().len() != value.len()
        || value.chars().any(char::is_control)
    {
        return Err(crate::Error::Validation(format!(
            "{field} must be non-empty, at most {max_bytes} bytes, and free of padding or control characters"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_validation_and_debug_are_bounded() {
        assert!(AuditContext::user("").is_err());
        assert!(AuditContext::service(" padded ").is_err());
        assert!(AuditContext::system("line\nbreak").is_err());
        let context = AuditContext::user("principal-secret-marker")
            .expect("valid actor")
            .with_correlation_id("request-secret-marker")
            .expect("valid correlation");
        let debug = format!("{context:?}");
        assert!(!debug.contains("principal-secret-marker"));
        assert!(!debug.contains("request-secret-marker"));
    }

    #[tokio::test]
    async fn metadata_binds_actor_and_active_tenant() {
        let context = AuditContext::service("worker-7").expect("valid actor");
        let metadata = crate::with_tenant("tenant-a", async {
            with_audit_context(context, async { current_metadata() }).await
        })
        .await
        .expect("metadata");
        assert_eq!(metadata.actor_kind, "service");
        assert_eq!(metadata.actor_id, "worker-7");
        assert_eq!(metadata.tenant_key.as_deref(), Some("string:tenant-a"));
    }

    #[test]
    fn accessors_and_debug_expose_only_safe_metadata() {
        let context = AuditContext::user("alice").expect("valid actor");
        assert_eq!(context.actor_kind(), AuditActorKind::User);
        assert_eq!(context.actor_id(), "alice");
        assert_eq!(context.correlation_id(), None);
        let context = context
            .with_correlation_id("req-1")
            .expect("valid correlation");
        assert_eq!(context.correlation_id(), Some("req-1"));
        assert_eq!(
            format!("{context:?}"),
            "AuditContext { actor_kind: User, actor_id_bytes: 5, has_correlation_id: true, reverted_audit_id: None, has_reason: false }"
        );
    }

    #[test]
    fn text_fields_accept_exactly_their_byte_limit() {
        let longest = "a".repeat(255);
        assert_eq!(
            AuditContext::user(longest.clone())
                .expect("255 bytes")
                .actor_id(),
            longest
        );
        let error = AuditContext::user("a".repeat(256)).expect_err("256 bytes");
        assert_eq!(
            error.to_string(),
            "Validation error: audit actor ID must be non-empty, at most 255 bytes, and free of padding or control characters"
        );
    }

    #[tokio::test]
    async fn revision_restores_need_a_positive_audit_id() {
        let context = AuditContext::system("restorer").expect("valid actor");
        let error = context
            .clone()
            .for_revision_restore(0, "undo")
            .expect_err("zero id");
        assert_eq!(
            error.to_string(),
            "Validation error: restored audit ID must be positive"
        );
        let restoring = context.for_revision_restore(1, "undo").expect("id 1");
        let metadata = with_audit_context(restoring, async { current_metadata() })
            .await
            .expect("metadata");
        assert_eq!(metadata.reverted_audit_id, Some(1));
        assert_eq!(metadata.reason.as_deref(), Some("undo"));
        assert_eq!(metadata.tenant_key, None);
    }

    #[tokio::test]
    async fn tenant_keys_encode_each_scalar_type_within_their_bound() {
        let key = |tenant: RullstValue| crate::with_tenant(tenant, async { active_tenant_key() });
        assert_eq!(
            key(RullstValue::Int(7)).await.unwrap().as_deref(),
            Some("i32:7")
        );
        assert_eq!(
            key(RullstValue::Bool(true)).await.unwrap().as_deref(),
            Some("bool:true")
        );
        assert_eq!(
            key(RullstValue::Float(1.5)).await.unwrap().as_deref(),
            Some("f64:3ff8000000000000")
        );
        let nan = key(RullstValue::Float(f64::NAN)).await;
        assert!(
            matches!(&nan, Err(crate::Error::Validation(message)) if message == "audit tenant ID must be finite"),
            "{nan:?}"
        );
        // `string:` takes 7 of the 512 key bytes.
        let longest = "t".repeat(505);
        assert_eq!(
            key(RullstValue::String(longest.clone())).await.unwrap(),
            Some(format!("string:{longest}"))
        );
        let too_long = key(RullstValue::String("t".repeat(506))).await;
        assert!(
            matches!(&too_long, Err(crate::Error::Validation(message)) if message.starts_with("audit tenant ID must be non-empty, at most 505 bytes")),
            "{too_long:?}"
        );
    }

    #[tokio::test]
    async fn wide_and_null_tenants_have_their_own_audit_keys() {
        let wide = crate::with_tenant(5_000_000_000_i64, async { active_tenant_key() }).await;
        assert_eq!(wide.unwrap().as_deref(), Some("i64:5000000000"));
        let null = crate::with_tenant(None::<i64>, async { active_tenant_key() }).await;
        assert!(
            matches!(&null, Err(crate::Error::Validation(message)) if message == "audit tenant ID must not be NULL"),
            "{null:?}"
        );
        let date = chrono::NaiveDate::from_ymd_opt(2026, 10, 15).expect("date");
        let dated = crate::with_tenant(date, async { active_tenant_key() }).await;
        assert!(
            matches!(&dated, Err(crate::Error::Validation(message)) if message == "audit tenant ID must not be a date or time"),
            "{dated:?}"
        );
    }
}
