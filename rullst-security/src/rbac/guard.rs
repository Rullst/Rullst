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
    pub fn authorize(ctx: &UserContext, required_role: &str) -> Result<(), String> {
        if ctx.has_role(required_role) || ctx.has_role("admin") {
            Ok(())
        } else {
            let store = crate::telemetry::SecurityStore::global();
            store.inc_rbac_denials();
            if let Ok(mut events) = store.live_events.lock() {
                events.insert(
                    0,
                    crate::telemetry::LiveSecurityEvent {
                        event_type: "RBAC_DENIAL".to_string(),
                        details: format!("User {} denied access requiring role '{}'", ctx.user_id, required_role),
                        client_ip: "127.0.0.1".to_string(),
                        timestamp_str: crate::telemetry::current_timestamp_str(),
                        verified_hmac: true,
                    },
                );
            }
            Err(format!("Access Denied: Required role '{}'", required_role))
        }
    }

    pub fn authorize_owner_or_role(
        ctx: &UserContext,
        resource_owner_id: &str,
        required_role: &str,
    ) -> Result<(), String> {
        if ctx.is_owner_of(resource_owner_id)
            || ctx.has_role(required_role)
            || ctx.has_role("admin")
        {
            Ok(())
        } else {
            let store = crate::telemetry::SecurityStore::global();
            store.inc_rbac_denials();
            if let Ok(mut events) = store.live_events.lock() {
                events.insert(
                    0,
                    crate::telemetry::LiveSecurityEvent {
                        event_type: "RBAC_DENIAL".to_string(),
                        details: format!("User {} denied ownership/role access for resource owner {}", ctx.user_id, resource_owner_id),
                        client_ip: "127.0.0.1".to_string(),
                        timestamp_str: crate::telemetry::current_timestamp_str(),
                        verified_hmac: true,
                    },
                );
            }
            Err("Access Denied: Insufficient permissions or ownership".to_string())
        }
    }
}
