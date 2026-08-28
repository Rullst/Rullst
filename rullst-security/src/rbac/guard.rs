use crate::error::SecurityError;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct UserContext {
    pub user_id: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
}

impl UserContext {
    pub fn new(user_id: impl Into<String>, roles: Vec<String>) -> Self {
        Self {
            user_id: user_id.into(),
            roles,
            permissions: Vec::new(),
            tenant_id: None,
        }
    }

    pub fn with_permissions(mut self, permissions: Vec<String>) -> Self {
        self.permissions = permissions;
        self
    }

    /// Binds this authenticated identity to one validated active tenant.
    ///
    /// Applications must derive this value from trusted membership state. A
    /// request header alone is never sufficient evidence of membership.
    pub fn try_with_tenant_id(
        mut self,
        tenant_id: impl Into<String>,
    ) -> Result<Self, SecurityError> {
        let tenant_id = tenant_id.into();
        if !valid_tenant_id(&tenant_id) {
            return Err(SecurityError::Unauthorized(
                "authenticated tenant identifier is invalid".to_string(),
            ));
        }
        self.tenant_id = Some(tenant_id);
        Ok(self)
    }

    pub fn tenant_id(&self) -> Option<&str> {
        self.tenant_id.as_deref()
    }

    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r.eq_ignore_ascii_case(role))
    }

    pub fn has_permission(&self, perm: &str) -> bool {
        self.permissions
            .iter()
            .any(|p| p.eq_ignore_ascii_case(perm))
    }

    pub fn is_owner_of(&self, resource_owner_id: &str) -> bool {
        self.user_id == resource_owner_id
    }

    pub fn is_in_tenant(&self, resource_tenant_id: &str) -> bool {
        self.tenant_id() == Some(resource_tenant_id)
    }
}

fn valid_tenant_id(tenant_id: &str) -> bool {
    !tenant_id.is_empty()
        && tenant_id.len() <= 128
        && tenant_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

pub struct RbacGuard;

impl RbacGuard {
    /// Requires an exact tenant match. Roles, including `admin`, never bypass
    /// this boundary.
    pub fn authorize_tenant(
        ctx: &UserContext,
        resource_tenant_id: &str,
    ) -> Result<(), SecurityError> {
        if valid_tenant_id(resource_tenant_id) && ctx.is_in_tenant(resource_tenant_id) {
            Ok(())
        } else {
            record_denial(ctx, "tenant-scoped resource access denied".to_string());
            Err(SecurityError::Forbidden(
                "Access Denied: Tenant context does not match the resource".to_string(),
            ))
        }
    }

    pub fn authorize(ctx: &UserContext, required_role: &str) -> Result<(), SecurityError> {
        if ctx.has_role(required_role) || ctx.has_role("admin") {
            Ok(())
        } else {
            let store = crate::telemetry::SecurityStore::global();
            store.inc_rbac_denials();
            store.push_local_event(crate::telemetry::LiveSecurityEvent::local(
                "RBAC_DENIAL",
                format!(
                    "User {} denied access requiring role '{}'",
                    ctx.user_id, required_role
                ),
                "unknown",
            ));
            Err(SecurityError::Forbidden(format!(
                "Access Denied: Required role '{}'",
                required_role
            )))
        }
    }

    pub fn authorize_owner_or_role(
        ctx: &UserContext,
        resource_owner_id: &str,
        required_role: &str,
    ) -> Result<(), SecurityError> {
        if ctx.is_owner_of(resource_owner_id)
            || ctx.has_role(required_role)
            || ctx.has_role("admin")
        {
            Ok(())
        } else {
            let store = crate::telemetry::SecurityStore::global();
            store.inc_rbac_denials();
            store.push_local_event(crate::telemetry::LiveSecurityEvent::local(
                "RBAC_DENIAL",
                format!(
                    "User {} denied ownership/role access for resource owner {}",
                    ctx.user_id, resource_owner_id
                ),
                "unknown",
            ));
            Err(SecurityError::Forbidden(
                "Access Denied: Insufficient permissions or ownership".to_string(),
            ))
        }
    }

    /// Requires the tenant boundary first, then applies owner/role policy.
    pub fn authorize_tenant_owner_or_role(
        ctx: &UserContext,
        resource_tenant_id: &str,
        resource_owner_id: &str,
        required_role: &str,
    ) -> Result<(), SecurityError> {
        Self::authorize_tenant(ctx, resource_tenant_id)?;
        Self::authorize_owner_or_role(ctx, resource_owner_id, required_role)
    }
}

fn record_denial(ctx: &UserContext, message: String) {
    let store = crate::telemetry::SecurityStore::global();
    store.inc_rbac_denials();
    store.push_local_event(crate::telemetry::LiveSecurityEvent::local(
        "RBAC_DENIAL",
        format!("User {}: {message}", ctx.user_id),
        "unknown",
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user() -> UserContext {
        UserContext::new("user-17", vec!["Editor".to_string()])
            .with_permissions(vec!["Posts:Publish".to_string()])
    }

    #[test]
    fn context_checks_roles_permissions_and_ownership() {
        let context = user();
        assert!(context.has_role("editor"));
        assert!(context.has_permission("posts:publish"));
        assert!(context.is_owner_of("user-17"));
        assert!(!context.has_role("auditor"));
        assert!(!context.has_permission("posts:delete"));
        assert!(!context.is_owner_of("other-user"));
        assert_eq!(context.tenant_id(), None);
    }

    #[test]
    fn tenant_context_is_validated_and_exact() {
        let context = user()
            .try_with_tenant_id("school-17")
            .expect("valid tenant context");
        assert_eq!(context.tenant_id(), Some("school-17"));
        assert!(context.is_in_tenant("school-17"));
        assert!(!context.is_in_tenant("school-18"));
        assert!(matches!(
            user().try_with_tenant_id("../school"),
            Err(SecurityError::Unauthorized(_))
        ));
    }

    #[test]
    fn role_authorization_accepts_required_role_or_admin_and_denies_others() {
        assert!(RbacGuard::authorize(&user(), "EDITOR").is_ok());
        let admin = UserContext::new("admin-1", vec!["ADMIN".to_string()]);
        assert!(RbacGuard::authorize(&admin, "auditor").is_ok());

        let denied = RbacGuard::authorize(&user(), "auditor");
        assert!(matches!(denied, Err(SecurityError::Forbidden(_))));
    }

    #[test]
    fn ownership_authorization_covers_owner_role_admin_and_denial() {
        assert!(RbacGuard::authorize_owner_or_role(&user(), "user-17", "moderator").is_ok());
        assert!(RbacGuard::authorize_owner_or_role(&user(), "other-user", "editor").is_ok());
        let admin = UserContext::new("admin-1", vec!["admin".to_string()]);
        assert!(RbacGuard::authorize_owner_or_role(&admin, "other-user", "moderator").is_ok());
        assert!(matches!(
            RbacGuard::authorize_owner_or_role(&user(), "other-user", "moderator"),
            Err(SecurityError::Forbidden(_))
        ));
    }

    #[test]
    fn tenant_authorization_never_allows_role_bypass() {
        let owner = UserContext::new("owner-1", vec!["member".to_string()])
            .try_with_tenant_id("school-1")
            .expect("tenant owner");
        let foreign_admin = UserContext::new("admin-2", vec!["admin".to_string()])
            .try_with_tenant_id("school-2")
            .expect("tenant admin");

        assert!(RbacGuard::authorize_tenant(&owner, "school-1").is_ok());
        assert!(matches!(
            RbacGuard::authorize_tenant(&foreign_admin, "school-1"),
            Err(SecurityError::Forbidden(_))
        ));
        assert!(
            RbacGuard::authorize_tenant_owner_or_role(&owner, "school-1", "owner-1", "admin",)
                .is_ok()
        );
        assert!(matches!(
            RbacGuard::authorize_tenant_owner_or_role(
                &foreign_admin,
                "school-1",
                "owner-1",
                "admin",
            ),
            Err(SecurityError::Forbidden(_))
        ));
    }
}
