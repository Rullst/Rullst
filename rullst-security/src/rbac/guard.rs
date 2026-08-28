use crate::error::SecurityError;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserContext {
    pub user_id: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

impl UserContext {
    pub fn new(user_id: impl Into<String>, roles: Vec<String>) -> Self {
        Self {
            user_id: user_id.into(),
            roles,
            permissions: Vec::new(),
        }
    }

    pub fn with_permissions(mut self, permissions: Vec<String>) -> Self {
        self.permissions = permissions;
        self
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
}

pub struct RbacGuard;

impl RbacGuard {
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
}
